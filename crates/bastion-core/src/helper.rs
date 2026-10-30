//! Helper privilégié : escalade root **unique** par session.
//!
//! Principe ergonomique et de sécurité :
//! 1. L'interface (utilisateur) lance une seule fois `pkexec <app> --serve …`
//!    au premier besoin de droits ; polkit demande le mot de passe **une fois**.
//! 2. Le helper tourne en root, écoute sur une socket UNIX dédiée, dans un
//!    répertoire appartenant à l'utilisateur, et n'exécute que les commandes
//!    de la liste blanche [`crate::exec::validate`].
//! 3. La socket n'accepte que l'utilisateur qui a lancé l'application
//!    (permissions 0600 + vérification `SO_PEERCRED`).
//! 4. Le helper s'arrête tout seul après une période d'inactivité.
//!
//! Le protocole est du JSON ligne par ligne, ce qui le rend trivial à auditer.

use crate::error::CoreError;
use crate::exec::{self, CommandSpec, Program};
use crate::transport::{ExecOutput, Transport};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

const PROTOCOL_VERSION: u32 = 1;

/// Requête envoyée par l'interface au helper.
#[derive(Debug, Serialize, Deserialize)]
pub struct HelperRequest {
    pub id: u64,
    /// Nom du programme (résolu via la liste blanche) ou « ping ».
    pub program: String,
    pub args: Vec<String>,
}

/// Réponse du helper.
#[derive(Debug, Serialize, Deserialize)]
pub struct HelperResponse {
    pub id: u64,
    pub ok: bool,
    pub error: Option<String>,
    #[serde(default)]
    pub code: i32,
    #[serde(default)]
    pub stdout: String,
    #[serde(default)]
    pub stderr: String,
}

/// Calcule le chemin de la socket pour un utilisateur donné.
pub fn socket_path_for(uid: u32) -> PathBuf {
    let run_user = PathBuf::from("/run/user").join(uid.to_string());
    if run_user.is_dir() {
        run_user.join("bastion.sock")
    } else {
        std::env::temp_dir().join(format!("bastion-{}.sock", uid))
    }
}

// ---------------------------------------------------------------------------
// Serveur (exécuté en root via pkexec par `bastion --serve`)
// ---------------------------------------------------------------------------

/// Point d'entrée du mode `--serve` (bloquant).
///
/// * `allowed_uid` : utilisateur autorisé à se connecter (l'invoker pkexec).
/// * `idle_timeout` : arrêt automatique après cette durée sans activité.
pub fn run_helper_server(socket: &Path, allowed_uid: u32, idle_timeout: std::time::Duration) -> Result<(), String> {
    // Prépare le répertoire parent (peut manquer hors session systemd).
    if let Some(parent) = socket.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("création de {} : {e}", parent.display()))?;
    }

    // Nettoie une socket périmée.
    if socket.exists() {
        let stale = UnixStream::connect(socket).is_err();
        if stale {
            let _ = std::fs::remove_file(socket);
        } else {
            // Une instance vit déjà : ne rien faire (l'autre helper sert la socket).
            return Ok(());
        }
    }

    let listener = UnixListener::bind(socket).map_err(|e| format!("bind {} : {e}", socket.display()))?;
    // Accept non bloquant : la boucle surveille aussi l'inactivité
    // (un accept bloquant empêcherait tout arrêt automatique).
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("nonblocking : {e}"))?;

    // Permissions strictes : utilisateur seul + root.
    let _ = std::fs::set_permissions(socket, std::fs::Permissions::from_mode(0o600));
    let _ = nix::unistd::chown(
        socket,
        Some(nix::unistd::Uid::from_raw(allowed_uid)),
        Some(nix::unistd::Gid::from_raw(allowed_uid)),
    );

    let deadline = Arc::new(std::sync::Mutex::new(std::time::Instant::now() + idle_timeout));

    loop {
        // Arrêt après inactivité prolongée.
        if std::time::Instant::now() > *deadline.lock().unwrap() {
            break;
        }
        match listener.accept() {
            Ok((stream, _)) => {
                let deadline = deadline.clone();
                // Vérifie l'identité du pair (SO_PEERCRED).
                let cred = nix::sys::socket::getsockopt(
                    &stream,
                    nix::sys::socket::sockopt::PeerCredentials,
                );
                let allowed = matches!(&cred, Ok(c) if c.uid() == allowed_uid);
                if !allowed {
                    continue; // connexion silencieusement ignorée
                }
                *deadline.lock().unwrap() = std::time::Instant::now() + idle_timeout;
                std::thread::spawn(move || {
                    let _ = serve_connection(stream);
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(250));
            }
            Err(_) => {
                std::thread::sleep(std::time::Duration::from_millis(250));
            }
        }
    }

    let _ = std::fs::remove_file(socket);
    Ok(())
}

fn serve_connection(stream: UnixStream) -> std::io::Result<()> {
    let mut writer = stream.try_clone()?;
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let resp = match serde_json::from_str::<HelperRequest>(&line) {
            Ok(req) => handle_request(&req),
            Err(e) => HelperResponse {
                id: 0,
                ok: false,
                error: Some(format!("requête invalide : {e}")),
                code: -1,
                stdout: String::new(),
                stderr: String::new(),
            },
        };
        let mut out = serde_json::to_string(&resp).map_err(std::io::Error::other)?;
        out.push('\n');
        writer.write_all(out.as_bytes())?;
        writer.flush()?;
    }
    Ok(())
}

fn handle_request(req: &HelperRequest) -> HelperResponse {
    // Ping : sonde de vie, sans exécution.
    if req.program == "ping" {
        return HelperResponse {
            id: req.id,
            ok: true,
            error: None,
            code: 0,
            stdout: format!("pong {PROTOCOL_VERSION}"),
            stderr: String::new(),
        };
    }

    let Some(program) = Program::from_name(&req.program) else {
        return HelperResponse {
            id: req.id,
            ok: false,
            error: Some(format!("programme « {} » non autorisé", req.program)),
            code: -1,
            stdout: String::new(),
            stderr: String::new(),
        };
    };

    let spec = CommandSpec {
        program,
        args: req.args.clone(),
    };
    if let Err(reason) = exec::validate(&spec) {
        return HelperResponse {
            id: req.id,
            ok: false,
            error: Some(format!("refusé par la liste blanche : {reason}")),
            code: -1,
            stdout: String::new(),
            stderr: String::new(),
        };
    }

    let Some(path) = exec::resolve_local(program) else {
        return HelperResponse {
            id: req.id,
            ok: false,
            error: Some(format!("programme « {} » introuvable", program.name())),
            code: -1,
            stdout: String::new(),
            stderr: String::new(),
        };
    };

    match std::process::Command::new(&path).args(&spec.args).output() {
        Ok(out) => HelperResponse {
            id: req.id,
            ok: true,
            error: None,
            code: out.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        },
        Err(e) => HelperResponse {
            id: req.id,
            ok: false,
            error: Some(format!("exécution : {e}")),
            code: -1,
            stdout: String::new(),
            stderr: String::new(),
        },
    }
}

// ---------------------------------------------------------------------------
// Client (côté interface)
// ---------------------------------------------------------------------------

/// Transport parlant au helper local.
#[derive(Debug)]
pub struct HelperTransport {
    socket: PathBuf,
    stream: tokio::sync::Mutex<tokio::net::UnixStream>,
    counter: AtomicU64,
}

impl HelperTransport {
    pub async fn connect(socket: &Path) -> Result<HelperTransport, CoreError> {
        let stream = tokio::net::UnixStream::connect(socket)
            .await
            .map_err(|e| CoreError::Disconnected(format!("helper injoignable : {e}")))?;
        Ok(HelperTransport {
            socket: socket.to_path_buf(),
            stream: tokio::sync::Mutex::new(stream),
            counter: AtomicU64::new(1),
        })
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket
    }
}

#[async_trait]
impl Transport for HelperTransport {
    async fn exec(&self, spec: &CommandSpec) -> Result<ExecOutput, CoreError> {
        let id = self.counter.fetch_add(1, Ordering::Relaxed);
        let req = HelperRequest {
            id,
            program: spec.program.name().to_string(),
            args: spec.args.clone(),
        };
        let mut line = serde_json::to_string(&req)
            .map_err(|e| CoreError::Other(format!("encodage : {e}")))?;
        line.push('\n');

        let mut stream = self.stream.lock().await;
        use tokio::io::AsyncWriteExt;
        stream
            .write_all(line.as_bytes())
            .await
            .map_err(|e| CoreError::Disconnected(format!("helper : {e}")))?;

        use tokio::io::AsyncBufReadExt;
        let mut reader = tokio::io::BufReader::new(&mut *stream);
        let mut resp_line = String::new();
        let resp: HelperResponse = tokio::time::timeout(
            crate::transport::command_timeout(&spec.program),
            async {
                loop {
                    resp_line.clear();
                    let n = reader
                        .read_line(&mut resp_line)
                        .await
                        .map_err(|e| CoreError::Disconnected(format!("helper : {e}")))?;
                    if n == 0 {
                        return Err(CoreError::Disconnected(
                            "le helper a fermé la connexion".into(),
                        ));
                    }
                    if let Ok(resp) = serde_json::from_str::<HelperResponse>(resp_line.trim()) {
                        if resp.id >= id {
                            return Ok(resp);
                        }
                    }
                    // Ligne périmée (reprise après déconnexion) : on continue.
                }
            },
        )
        .await
        .map_err(|_| CoreError::Other("délai dépassé en attente du helper".into()))??;

        if !resp.ok {
            return Err(CoreError::Other(
                resp.error.unwrap_or_else(|| "erreur du helper".into()),
            ));
        }
        Ok(ExecOutput {
            stdout: resp.stdout,
            stderr: resp.stderr,
            code: resp.code,
        })
    }

    fn describe(&self) -> String {
        "Local (helper)".to_string()
    }
}

/// Teste si une socket helper répond au ping.
pub async fn ping_helper(socket: &Path) -> bool {
    let Ok(mut stream) = tokio::net::UnixStream::connect(socket).await else {
        return false;
    };
    let req = HelperRequest {
        id: 1,
        program: "ping".into(),
        args: vec![],
    };
    let Ok(mut line) = serde_json::to_string(&req) else {
        return false;
    };
    line.push('\n');
    use tokio::io::AsyncWriteExt;
    if stream.write_all(line.as_bytes()).await.is_err() {
        return false;
    }
    let mut buf = String::new();
    use tokio::io::AsyncBufReadExt;
    let mut reader = tokio::io::BufReader::new(stream);
    matches!(
        tokio::time::timeout(
            std::time::Duration::from_secs(3),
            reader.read_line(&mut buf)
        )
        .await,
        Ok(Ok(n)) if n > 0 && buf.contains("pong")
    )
}

/// Lance le helper via pkexec (si nécessaire) et attend qu'il réponde.
///
/// `app_exe` : chemin du binaire de l'application (lui-même sert de helper).
/// Retourne un transport prêt.
///
/// Deux points critiques :
/// 1. Le binaire courant peut vivre dans un montage FUSE volatile
///    (AppImage : /tmp/.mount_*) que root ne peut pas parcourir — pkexec
///    échouerait avec « Permission denied ». On le copie donc dans un
///    emplacement stable appartenant à l'utilisateur ([`stage_helper`]).
/// 2. Un refus polkit (annulation, agent absent…) doit se voir
///    immédiatement : on surveille le code de sortie de pkexec et on
///    remonte son stderr tel quel, au lieu d'attendre bêtement un délai.
pub async fn spawn_and_connect(
    app_exe: &Path,
    socket: &Path,
    wait: std::time::Duration,
) -> Result<HelperTransport, CoreError> {
    if ping_helper(socket).await {
        return HelperTransport::connect(socket).await;
    }

    let pkexec = pkexec_bin().ok_or_else(|| {
        CoreError::Other(
            "pkexec est introuvable : installez policykit-1 \
             (ou lancez l'application en root, ou utilisez une connexion SSH)."
                .into(),
        )
    })?;

    let staged = stage_helper(app_exe);
    let uid = nix::unistd::Uid::effective().as_raw();

    let mut child = tokio::process::Command::new(pkexec)
        .arg(&staged)
        .arg("--serve")
        .arg("--socket")
        .arg(socket)
        .arg("--uid")
        .arg(uid.to_string())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| CoreError::Other(format!("lancement de pkexec : {e}")))?;

    // pkexec reste vivant tant que le helper tourne : sa sortie prématurée
    // (authentification refusée/annulée, erreur d'accès…) est un échec.
    let start = std::time::Instant::now();
    loop {
        if ping_helper(socket).await {
            return HelperTransport::connect(socket).await;
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut stderr = String::new();
                if let Some(mut out) = child.stderr.take() {
                    use tokio::io::AsyncReadExt;
                    let _ = out.read_to_string(&mut stderr).await;
                }
                let msg = stderr.trim();
                if status.success() {
                    // Sorti « proprement » sans socket : probablement une
                    // instance déjà vivante détectée par le helper.
                    if ping_helper(socket).await {
                        return HelperTransport::connect(socket).await;
                    }
                    return Err(CoreError::Other(
                        "l'assistant root s'est arrêté immédiatement".into(),
                    ));
                }
                return Err(CoreError::Other(format!(
                    "pkexec a refusé (code {status}) : {}",
                    if msg.is_empty() { "aucun message" } else { msg }
                )));
            }
            Ok(None) => {}
            Err(e) => {
                return Err(CoreError::Other(format!("surveillance de pkexec : {e}")));
            }
        }
        if start.elapsed() > wait {
            let _ = child.start_kill();
            return Err(CoreError::Other(format!(
                "l'assistant root n'a pas démarré en {} s — \
                 la fenêtre de mot de passe est-elle apparue ?",
                wait.as_secs()
            )));
        }
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }
}

/// Copie le binaire hors d'un montage volatile (AppImage) vers
/// `~/.cache/bastion/helper/`, accessible à root pour l'exec pkexec.
/// Hors AppImage (installation classique), le chemin est retourné tel quel.
pub fn stage_helper(app_exe: &Path) -> PathBuf {
    let exe_str = app_exe.to_string_lossy();
    let volatile = exe_str.contains(".mount_") || app_exe.starts_with("/tmp/.mount_");
    if !volatile {
        return app_exe.to_path_buf();
    }

    let Some(dir) = staging_dir() else {
        return app_exe.to_path_buf();
    };
    let staged = dir.join(app_exe.file_name().unwrap_or_else(|| std::ffi::OsStr::new("bastion")));

    let src_len = std::fs::metadata(app_exe).map(|m| m.len()).unwrap_or(0);
    let ok = match std::fs::metadata(&staged) {
        Ok(m) => m.len() == src_len,
        Err(_) => false,
    };
    if !ok {
        // Copie atomique : d'abord un fichier temporaire, puis rename.
        let tmp = staged.with_extension("new");
        if std::fs::copy(app_exe, &tmp).is_ok() {
            let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755));
            if std::fs::rename(&tmp, &staged).is_err() {
                let _ = std::fs::remove_file(&tmp);
            }
        }
    }
    if staged.is_file() {
        staged
    } else {
        app_exe.to_path_buf()
    }
}

fn staging_dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache"))
        })?;
    let dir = base.join("bastion").join("helper");
    std::fs::create_dir_all(&dir).ok()?;
    let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
    Some(dir)
}

fn pkexec_bin() -> Option<PathBuf> {
    ["/usr/bin/pkexec", "/bin/pkexec"]
        .iter()
        .map(PathBuf::from)
        .find(|p| p.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Le test du protocole complet (serveur + client) est fait au niveau du
    // binaire : ici on vérifie le protocole en mémoire.
    #[test]
    fn protocole_serveur_refuse_le_hors_liste() {
        let req = HelperRequest {
            id: 7,
            program: "sh".into(),
            args: vec!["-c".into(), "id".into()],
        };
        let resp = handle_request(&req);
        assert!(!resp.ok);
        assert!(resp.error.unwrap().contains("non autorisé"));
    }

    #[test]
    fn protocole_ping() {
        let req = HelperRequest {
            id: 1,
            program: "ping".into(),
            args: vec![],
        };
        let resp = handle_request(&req);
        assert!(resp.ok);
        assert!(resp.stdout.contains("pong"));
    }

    #[test]
    fn protocole_execute_refuse_arguments_bizarres() {
        // « status --nimporte » n'est pas dans la liste blanche.
        let req = HelperRequest {
            id: 2,
            program: "ufw".into(),
            args: vec!["status".into(), "--nimporte".into()],
        };
        let resp = handle_request(&req);
        assert!(!resp.ok);
        assert!(resp.error.unwrap().contains("liste blanche"));
    }
}

#[cfg(test)]
mod spawn_tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// pkexec refuse un chemin inexistant AVANT le dialogue polkit : la
    /// fonction doit échouer vite avec le message réel, sans attendre.
    #[tokio::test]
    async fn spawn_and_connect_echoue_vite_sur_chemin_invalide() {
        let socket = std::env::temp_dir().join(format!(
            "bastion-ff-{}-{}.sock",
            std::process::id(),
            line!()
        ));
        let _ = std::fs::remove_file(&socket);
        let start = Instant::now();
        let res = spawn_and_connect(
            Path::new("/definitivement/inexistant"),
            &socket,
            Duration::from_secs(30),
        )
        .await;
        let elapsed = start.elapsed();
        assert!(res.is_err(), "devrait échouer");
        let msg = res.unwrap_err().to_string();
        assert!(msg.contains("pkexec"), "message : {msg}");
        assert!(
            msg.contains("No such file") || msg.contains("Error accessing"),
            "message pkexec attendu : {msg}"
        );
        assert!(
            elapsed < Duration::from_secs(10),
            "échec trop lent : {elapsed:?}"
        );
        let _ = std::fs::remove_file(&socket);
    }

    /// Un binaire dans un montage AppImage (motif .mount_) doit être copié
    /// vers le répertoire de staging stable, avec son contenu intact.
    #[test]
    fn stage_helper_copie_le_binaire_du_montage() {
        // Chemin « volatile » au nom AppImage, parfaitement créable.
        let mount = std::env::temp_dir()
            .join(format!(".mount_TESTBASTION-{}-{}", std::process::id(), line!()));
        let bin = mount.join("usr/bin/bastion");
        let _ = std::fs::remove_dir_all(&mount);
        std::fs::create_dir_all(bin.parent().unwrap()).unwrap();
        std::fs::write(&bin, b"contenu factice du binaire pour test").unwrap();

        // Hors montage : inchangé.
        assert_eq!(
            stage_helper(Path::new("/usr/bin/bastion")),
            PathBuf::from("/usr/bin/bastion")
        );

        // Montage volatile : copié dans le cache, contenu identique.
        let staged = stage_helper(&bin);
        assert_ne!(staged, bin, "devrait être copié");
        assert!(staged.to_string_lossy().contains("bastion"), "chemin : {staged:?}");
        assert_eq!(
            std::fs::read(&staged).unwrap(),
            b"contenu factice du binaire pour test".to_vec()
        );

        // Idempotent : un second appel reprend le même chemin.
        let again = stage_helper(&bin);
        assert_eq!(again, staged);

        let _ = std::fs::remove_file(&staged);
        let _ = std::fs::remove_dir_all(&mount);
    }
}
