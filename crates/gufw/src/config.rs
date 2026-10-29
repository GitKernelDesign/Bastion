//! Configuration persistante de l'application (thème, réglages, profils SSH,
//! empreintes connues). Aucun mot de passe n'est jamais persisté ici.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeKind {
    Dark,
    Light,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProfileAuth {
    Password,
    Key { path: String },
}

/// Profil SSH enregistré. Les secrets ne font jamais partie du profil :
/// le mot de passe est saisi à chaque session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshProfileDef {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub auth: ProfileAuth,
    pub use_sudo: bool,
}

/// Empreinte mémorisée d'un serveur (TOFU).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownHost {
    pub host: String,
    pub port: u16,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub theme: ThemeKind,
    pub font_scale: f32,
    pub auto_refresh: bool,
    pub refresh_secs: u64,
    pub success_toasts: bool,
    pub profiles: Vec<SshProfileDef>,
    pub known_hosts: Vec<KnownHost>,
    pub last_profile: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            theme: ThemeKind::Dark,
            font_scale: 1.0,
            auto_refresh: true,
            refresh_secs: 5,
            success_toasts: true,
            profiles: Vec::new(),
            known_hosts: Vec::new(),
            last_profile: None,
        }
    }
}

impl AppConfig {
    pub fn config_dir() -> PathBuf {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(|h| PathBuf::from(h).join(".config"))
            })
            .unwrap_or_else(|| PathBuf::from(".config"));
        base.join("gufw")
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.json")
    }

    pub fn load() -> AppConfig {
        match std::fs::read_to_string(Self::config_path()) {
            Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
            Err(_) => AppConfig::default(),
        }
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Some(dir) = path.parent() {
            if std::fs::create_dir_all(dir).is_ok() {
                // Répertoire privé : il ne contient que des réglages et des
                // empreintes, mais rien ne justifie une lecture par autrui.
                let _ = std::fs::set_permissions(
                    dir,
                    std::fs::Permissions::from_mode(0o700),
                );
            }
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let tmp = path.with_extension("json.tmp");
            if std::fs::write(&tmp, json).is_ok() {
                let _ = std::fs::rename(&tmp, &path);
            }
        }
    }

    pub fn upsert_profile(&mut self, def: SshProfileDef) {
        if let Some(p) = self
            .profiles
            .iter_mut()
            .find(|p| p.name == def.name)
        {
            *p = def;
        } else {
            self.profiles.push(def);
        }
        self.save();
    }

    pub fn delete_profile(&mut self, name: &str) {
        self.profiles.retain(|p| p.name != name);
        if self.last_profile.as_deref() == Some(name) {
            self.last_profile = None;
        }
        self.save();
    }

    /// Empreinte connue pour un hôte, si elle existe (TOFU).
    pub fn known_fingerprint(&self, host: &str, port: u16) -> Option<String> {
        self.known_hosts
            .iter()
            .find(|k| k.host == host && k.port == port)
            .map(|k| k.fingerprint.clone())
    }

    pub fn remember_host(&mut self, host: &str, port: u16, fingerprint: &str) {
        if let Some(k) = self
            .known_hosts
            .iter_mut()
            .find(|k| k.host == host && k.port == port)
        {
            k.fingerprint = fingerprint.to_string();
        } else {
            self.known_hosts.push(KnownHost {
                host: host.to_string(),
                port,
                fingerprint: fingerprint.to_string(),
            });
        }
        self.save();
    }

}

use std::os::unix::fs::PermissionsExt;
