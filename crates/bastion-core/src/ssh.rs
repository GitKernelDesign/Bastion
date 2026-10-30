//! Transport SSH (crate `russh`, client SSH 100 % Rust embarqué dans
//! l'application — aucune dépendance système).
//!
//! Ergonomie de sécurité : validation TOFU de la clé du serveur.
//! - 1re connexion : l'empreinte SHA256 est remontée à l'interface, qui
//!   demande confirmation explicite avant de la mémoriser.
//! - Changement d'empreinte : avertissement fort (usurpation possible),
//!   l'utilisateur doit confirmer explicitement.

use crate::error::CoreError;
use crate::exec::CommandSpec;
use crate::transport::{ExecOutput, Transport};
use async_trait::async_trait;
use russh::keys::key;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;

/// Méthode d'authentification. Les mots de passe vivent en mémoire pour la
/// session uniquement (jamais persistés).
#[derive(Debug, Clone)]
pub enum SshAuth {
    Password(String),
    Key {
        path: PathBuf,
        passphrase: Option<String>,
    },
}

/// Paramètres d'une connexion SSH.
#[derive(Debug, Clone)]
pub struct SshParams {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub auth: SshAuth,
    /// Empreinte attendue (TOFU) : `Some` => vérification stricte.
    pub expected_fingerprint: Option<String>,
    /// Exécuter les commandes via sudo (serveurs sans accès root direct).
    pub use_sudo: bool,
    /// Mot de passe sudo (session uniquement), envoyé sur l'entrée standard.
    pub sudo_password: Option<String>,
}

/// Résultat d'une tentative de connexion : un transport prêt, ou une
/// décision d'interface requise (clé du serveur inconnue ou modifiée).
pub enum ConnectOutcome {
    Connected(Box<SshTransport>),
    /// Clé inconnue : l'interface doit faire confirmer l'empreinte.
    HostKeyUnknown { fingerprint: String },
    /// Clé différente de celle mémorisée : avertissement fort.
    HostKeyChanged { old: String, new: String },
}

struct SshHandler {
    expected: Option<String>,
    seen: Arc<Mutex<Option<String>>>,
}

impl SshHandler {
    fn new(expected: Option<String>, seen: Arc<Mutex<Option<String>>>) -> SshHandler {
        SshHandler { expected, seen }
    }
}

#[async_trait]
impl russh::client::Handler for SshHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &key::PublicKey,
    ) -> Result<bool, Self::Error> {
        let fp = format!("SHA256:{}", server_public_key.fingerprint());
        *self.seen.lock().unwrap() = Some(fp.clone());
        Ok(self.expected.as_deref() == Some(fp.as_str()))
    }
}

pub struct SshTransport {
    handle: AsyncMutex<russh::client::Handle<SshHandler>>,
    params: SshParams,
}

impl SshTransport {
    pub async fn connect(params: SshParams) -> Result<ConnectOutcome, CoreError> {
        let config = russh::client::Config {
            keepalive_interval: Some(std::time::Duration::from_secs(20)),
            keepalive_max: 3,
            inactivity_timeout: None,
            ..Default::default()
        };

        let seen = Arc::new(Mutex::new(None));
        let handler = SshHandler::new(params.expected_fingerprint.clone(), seen.clone());
        let addr = (params.host.as_str(), params.port);

        let handle = tokio::time::timeout(
            std::time::Duration::from_secs(25),
            russh::client::connect(Arc::new(config), addr, handler),
        )
        .await
        .map_err(|_| CoreError::Disconnected(format!("{} : délai de connexion dépassé", params.host)))?;

        let mut handle = match handle {
            Ok(h) => h,
            Err(e) => {
                // Clé refusée ? Le handler a mémorisé l'empreinte présentée.
                let presented = seen.lock().unwrap().clone();
                if let Some(fp) = presented {
                    return match &params.expected_fingerprint {
                        None => Ok(ConnectOutcome::HostKeyUnknown { fingerprint: fp }),
                        Some(old) if old != &fp => Ok(ConnectOutcome::HostKeyChanged {
                            old: old.clone(),
                            new: fp,
                        }),
                        Some(_) => Err(CoreError::Other(format!("SSH : {e}"))),
                    };
                }
                return Err(CoreError::Disconnected(format!(
                    "{} : {e}",
                    params.host
                )));
            }
        };

        let auth_ok = match &params.auth {
            SshAuth::Password(pw) => handle
                .authenticate_password(params.user.clone(), pw.clone())
                .await
                .map_err(|e| CoreError::Disconnected(format!("SSH : {e}")))?,
            SshAuth::Key { path, passphrase } => {
                let kp =
                    russh::keys::load_secret_key(path, passphrase.as_deref()).map_err(|e| {
                        CoreError::Other(format!("clé privée {} : {e}", path.display()))
                    })?;
                handle
                    .authenticate_publickey(params.user.clone(), Arc::new(kp))
                    .await
                    .map_err(|e| CoreError::Disconnected(format!("SSH : {e}")))?
            }
        };

        if !auth_ok {
            return Err(CoreError::SshAuthFailed {
                user: params.user.clone(),
            });
        }

        Ok(ConnectOutcome::Connected(Box::new(SshTransport {
            handle: AsyncMutex::new(handle),
            params,
        })))
    }

    pub fn params(&self) -> &SshParams {
        &self.params
    }

    fn wrap_sudo(&self, cmd: &str) -> String {
        if !self.params.use_sudo {
            return cmd.to_string();
        }
        if self.params.sudo_password.is_some() {
            format!("sudo -S -p '' -- {cmd}")
        } else {
            format!("sudo -n -- {cmd}")
        }
    }
}

#[async_trait]
impl Transport for SshTransport {
    async fn exec(&self, spec: &CommandSpec) -> Result<ExecOutput, CoreError> {
        let cmd = self.wrap_sudo(&spec.to_shell());

        let handle = self.handle.lock().await;
        if handle.is_closed() {
            return Err(CoreError::Disconnected("session SSH fermée".into()));
        }
        let mut channel = handle
            .channel_open_session()
            .await
            .map_err(|e| CoreError::Disconnected(format!("SSH : {e}")))?;

        channel
            .exec(true, cmd.as_bytes())
            .await
            .map_err(|e| CoreError::Disconnected(format!("SSH : {e}")))?;

        // Mot de passe sudo sur l'entrée standard, puis fermeture de stdin.
        if self.params.use_sudo {
            if let Some(pw) = &self.params.sudo_password {
                let mut data = pw.clone().into_bytes();
                data.push(b'\n');
                if let Err(e) = channel.data(data.as_slice()).await {
                    return Err(CoreError::Disconnected(format!("SSH : {e}")));
                }
                if let Err(e) = channel.eof().await {
                    return Err(CoreError::Disconnected(format!("SSH : {e}")));
                }
            }
        }

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut code = 0i32;

        let read_all = async {
            loop {
                match channel.wait().await {
                    Some(russh::ChannelMsg::Data { ref data }) => {
                        stdout.extend_from_slice(data);
                    }
                    Some(russh::ChannelMsg::ExtendedData { ref data, .. }) => {
                        stderr.extend_from_slice(data);
                    }
                    Some(russh::ChannelMsg::ExitStatus { exit_status }) => {
                        code = exit_status as i32;
                    }
                    Some(russh::ChannelMsg::Eof) | Some(russh::ChannelMsg::Close) => break,
                    None => break,
                    _ => {}
                }
            }
        };

        let limit = crate::transport::command_timeout(&spec.program);
        tokio::time::timeout(limit, read_all)
            .await
            .map_err(|_| CoreError::Disconnected("délai dépassé pendant l'exécution".into()))?;

        Ok(ExecOutput {
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
            code,
        })
    }

    async fn shutdown(&self) {
        let handle = self.handle.lock().await;
        let _ = handle
            .disconnect(russh::Disconnect::ByApplication, "Au revoir", "fr")
            .await;
    }

    fn describe(&self) -> String {
        format!("SSH : {}@{}", self.params.user, self.params.host)
    }
}
