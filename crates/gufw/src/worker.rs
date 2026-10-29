//! Worker : thread dédié au runtime asynchrone et à la connexion courante.
//! L'interface ne bloque jamais — elle envoie des jobs et collecte des
//! événements. Chaque commande exécutée est journalisée (transparence totale).

use async_trait::async_trait;
use gufw_core::op::{run_op, Op, OpResult};
use gufw_core::local::LocalRoot;
use gufw_core::ssh::{ConnectOutcome, SshParams, SshTransport};
use gufw_core::{
    CoreError, ExecOutput, Transport,
    helper::{HelperTransport, socket_path_for, spawn_and_connect},
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

/// Travail demandé par l'interface.
pub enum Job {
    ConnectLocal { app_exe: PathBuf },
    ConnectSsh { params: SshParams },
    Disconnect,
    Op { id: u64, op: Op },
    /// Détection locale (sans droits) des outils : versions ufw/fail2ban.
    ProbeLocalTools,
}

/// Entrée de journal (une commande exécutée).
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub at: SystemTime,
    pub command: String,
    pub ok: bool,
    pub code: i32,
    /// Extrait d'erreur (queue de stderr) pour l'affichage.
    pub detail: String,
}

/// Événements remontés à l'interface.
pub enum Event {
    Connected { desc: String },
    ConnectFailed { reason: String },
    HostKeyUnknown { host: String, fingerprint: String },
    HostKeyChanged { host: String, old: String, new: String },
    Disconnected { reason: String },
    OpDone { id: u64, result: Result<OpResult, CoreError> },
    Log(LogEntry),
    ProbeLocalTools { ufw: Option<String>, fail2ban: Option<String> },
}

/// Handle côté interface : envoi non bloquant, réception par sondage.
pub struct WorkerHandle {
    tx: std::sync::mpsc::Sender<Job>,
    rx: std::sync::mpsc::Receiver<Event>,
}

impl WorkerHandle {
    pub fn spawn() -> WorkerHandle {
        let (tx_jobs, rx_jobs) = std::sync::mpsc::channel::<Job>();
        let (tx_ev, rx_ev) = std::sync::mpsc::channel::<Event>();
        std::thread::Builder::new()
            .name("gufw-worker".into())
            .spawn(move || worker_main(rx_jobs, tx_ev))
            .expect("thread worker");
        WorkerHandle { tx: tx_jobs, rx: rx_ev }
    }

    pub fn send(&self, job: Job) {
        let _ = self.tx.send(job);
    }

    /// Vide la file d'événements (appelé à chaque frame).
    pub fn poll(&self) -> Vec<Event> {
        let mut out = Vec::new();
        while let Ok(ev) = self.rx.try_recv() {
            out.push(ev);
        }
        out
    }
}

/// Transport décorateur : journalise chaque commande.
struct LoggingTransport {
    inner: Arc<dyn Transport>,
    tx: std::sync::mpsc::Sender<Event>,
}

#[async_trait]
impl Transport for LoggingTransport {
    async fn exec(&self, spec: &gufw_core::exec::CommandSpec) -> Result<ExecOutput, CoreError> {
        let start = Instant::now();
        let res = self.inner.exec(spec).await;
        let elapsed = start.elapsed();
        let entry = match &res {
            Ok(o) => LogEntry {
                at: SystemTime::now(),
                command: spec.display(),
                ok: o.success(),
                code: o.code,
                detail: tail(&o.stderr, 300),
            },
            Err(e) => LogEntry {
                at: SystemTime::now(),
                command: spec.display(),
                ok: false,
                code: -1,
                detail: tail(&e.to_string(), 300),
            },
        };
        let _ = self.tx.send(Event::Log(entry));
        let _ = elapsed;
        res
    }

    async fn shutdown(&self) {
        self.inner.shutdown().await;
    }

    fn describe(&self) -> String {
        self.inner.describe()
    }
}

fn tail(s: &str, max: usize) -> String {
    let t = s.trim();
    if t.chars().count() <= max {
        t.to_string()
    } else {
        let cut: String = t.chars().skip(t.chars().count() - max).collect();
        format!("…{cut}")
    }
}

type SharedTransport = Arc<tokio::sync::RwLock<Option<Arc<LoggingTransport>>>>;

fn worker_main(rx_jobs: std::sync::mpsc::Receiver<Job>, tx_ev: std::sync::mpsc::Sender<Event>) {
    let rt = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(_) => return,
    };
    let transport: SharedTransport = Arc::new(tokio::sync::RwLock::new(None));

    // Boucle bloquante sur les jobs ; chaque job tourne dans le runtime.
    while let Ok(job) = rx_jobs.recv() {
        let t = transport.clone();
        let tx = tx_ev.clone();
        rt.spawn(async move {
            handle_job(t, tx, job).await;
        });
    }
}

async fn handle_job(transport: SharedTransport, tx: std::sync::mpsc::Sender<Event>, job: Job) {
    match job {
        Job::ConnectLocal { app_exe } => {
            let res = connect_local(&app_exe).await;
            finish_connect(transport, tx, res.map(|t| ("Local".to_string(), t))).await;
        }
        Job::ConnectSsh { params } => {
            let host = params.host.clone();
            match SshTransport::connect(params).await {
                Ok(ConnectOutcome::Connected(t)) => {
                    finish_connect(
                        transport,
                        tx,
                        Ok((format!("SSH {host}"), t as Box<dyn Transport>)),
                    )
                    .await;
                }
                Ok(ConnectOutcome::HostKeyUnknown { fingerprint }) => {
                    let _ = tx.send(Event::HostKeyUnknown {
                        host,
                        fingerprint,
                    });
                }
                Ok(ConnectOutcome::HostKeyChanged { old, new }) => {
                    let _ = tx.send(Event::HostKeyChanged { host, old, new });
                }
                Err(e) => {
                    let _ = tx.send(Event::ConnectFailed {
                        reason: e.to_string(),
                    });
                }
            }
        }
        Job::Disconnect => {
            let mut guard = transport.write().await;
            if let Some(t) = guard.take() {
                t.shutdown().await;
            }
            drop(guard);
            let _ = tx.send(Event::Disconnected {
                reason: "Déconnexion demandée".into(),
            });
        }
        Job::Op { id, op } => {
            let current = {
                let guard = transport.read().await;
                guard.clone()
            };
            let Some(t) = current else {
                let _ = tx.send(Event::OpDone {
                    id,
                    result: Err(CoreError::NotConnected),
                });
                return;
            };
            let res = run_op(t.as_ref() as &dyn Transport, &op).await;
            if let Err(e) = &res {
                if e.is_connection_loss() {
                    let mut guard = transport.write().await;
                    let _ = guard.take();
                    drop(guard);
                    let _ = tx.send(Event::Disconnected {
                        reason: e.to_string(),
                    });
                }
            }
            let _ = tx.send(Event::OpDone { id, result: res });
        }
        Job::ProbeLocalTools => {
            let ufw = probe_version(Path::new("/usr/sbin/ufw")).await;
            let f2b = probe_version(Path::new("/usr/bin/fail2ban-client")).await;
            let _ = tx.send(Event::ProbeLocalTools { ufw, fail2ban: f2b });
        }
    }
}

async fn probe_version(path: &Path) -> Option<String> {
    let out = tokio::time::timeout(
        Duration::from_secs(5),
        tokio::process::Command::new(path)
            .arg("--version")
            .output(),
    )
    .await
    .ok()?
    .ok()?;
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.lines().next().map(|l| l.trim().to_string())
}

async fn connect_local(app_exe: &Path) -> Result<Box<dyn Transport>, CoreError> {
    let uid = nix_euid();
    if uid == 0 {
        Ok(Box::new(LocalRoot))
    } else {
        let socket = socket_path_for(uid);
        let t = spawn_and_connect(app_exe, &socket, Duration::from_secs(90)).await?;
        Ok(Box::new(t))
    }
}

fn nix_euid() -> u32 {
    // Évite une dépendance directe : lit l'uid effectif via /proc.
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines().find_map(|l| {
                l.strip_prefix("Uid:")
                    .map(|rest| rest.split_whitespace().next().unwrap_or("0").to_string())
            })
        })
        .and_then(|u| u.parse().ok())
        .unwrap_or_else(|| std::env::var("PKEXEC_UID").ok().and_then(|s| s.parse().ok()).unwrap_or(0))
}

async fn finish_connect(
    transport: SharedTransport,
    tx: std::sync::mpsc::Sender<Event>,
    res: Result<(String, Box<dyn Transport>), CoreError>,
) {
    match res {
        Ok((_label, t)) => {
            let logging = LoggingTransport {
                inner: Arc::from(t),
                tx: tx.clone(),
            };
            let desc = logging.describe();
            *transport.write().await = Some(Arc::new(logging));
            let _ = tx.send(Event::Connected { desc });
        }
        Err(e) => {
            let _ = tx.send(Event::ConnectFailed {
                reason: e.to_string(),
            });
        }
    }
}

/// Connexion directe au helper déjà démarré (sans re-pkexec) : utilisée pour
/// les reconnexions rapides après inactivité du helper.
#[allow(dead_code)]
pub async fn reconnect_helper() -> Result<HelperTransport, CoreError> {
    let socket = socket_path_for(nix_euid());
    HelperTransport::connect(&socket).await
}
