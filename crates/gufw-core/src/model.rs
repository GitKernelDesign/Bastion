//! Modèles de données partagés entre le cœur et l'interface.
//!
//! Les champs textuels (« De », « Vers »…) conservent la formulation d'ufw ;
//! l'interface les reformule (« Anywhere » → « Partout ») à l'affichage.

use serde::{Deserialize, Serialize};

/// Sens d'une politique par défaut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    Incoming,
    Outgoing,
    Routed,
}

impl Direction {
    pub fn ufw_key(&self) -> &'static str {
        match self {
            Direction::Incoming => "incoming",
            Direction::Outgoing => "outgoing",
            Direction::Routed => "routed",
        }
    }
}

/// Politique d'ufw : autoriser, refuser silencieusement ou rejeter avec
/// notification à l'émetteur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Policy {
    Allow,
    Deny,
    Reject,
}

impl Policy {
    pub fn ufw_name(&self) -> &'static str {
        match self {
            Policy::Allow => "allow",
            Policy::Deny => "deny",
            Policy::Reject => "reject",
        }
    }

    pub fn from_ufw_name(s: &str) -> Option<Policy> {
        match s {
            "allow" => Some(Policy::Allow),
            "deny" => Some(Policy::Deny),
            "reject" => Some(Policy::Reject),
            _ => None,
        }
    }

    pub fn libelle(&self) -> &'static str {
        match self {
            Policy::Allow => "Autoriser",
            Policy::Deny => "Refuser",
            Policy::Reject => "Rejeter",
        }
    }
}

/// Niveau de journalisation UFW.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Off,
    Low,
    Medium,
    High,
    Full,
}

impl LogLevel {
    pub fn ufw_name(&self) -> &'static str {
        match self {
            LogLevel::Off => "off",
            LogLevel::Low => "low",
            LogLevel::Medium => "medium",
            LogLevel::High => "high",
            LogLevel::Full => "full",
        }
    }

    pub fn from_ufw_name(s: &str) -> Option<LogLevel> {
        match s {
            "off" => Some(LogLevel::Off),
            "low" => Some(LogLevel::Low),
            "medium" => Some(LogLevel::Medium),
            "high" => Some(LogLevel::High),
            "full" => Some(LogLevel::Full),
            _ => None,
        }
    }

    pub fn libelle(&self) -> &'static str {
        match self {
            LogLevel::Off => "Désactivée",
            LogLevel::Low => "Faible",
            LogLevel::Medium => "Moyenne",
            LogLevel::High => "Élevée",
            LogLevel::Full => "Complète",
        }
    }
}

/// Politiques par défaut telles que rapportées par `ufw status verbose`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Defaults {
    pub incoming: Option<Policy>,
    pub outgoing: Option<Policy>,
    pub routed: Option<Policy>,
    /// Le routage est affiché « disabled » par ufw quand le forwarding noyau
    /// est désactivé, indépendamment de la politique enregistrée.
    pub routed_disabled: bool,
}

/// État complet du pare-feu (sortie `ufw status verbose`).
#[derive(Debug, Clone, Default)]
pub struct FirewallStatus {
    pub active: bool,
    pub logging_on: bool,
    pub logging_level: Option<LogLevel>,
    pub defaults: Defaults,
    /// Sortie brute, pour la transparence (vue « détail »).
    pub raw: String,
}

/// Action d'une règle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleAction {
    Allow,
    Deny,
    Reject,
    Limit,
}

impl RuleAction {
    pub fn ufw_name(&self) -> &'static str {
        match self {
            RuleAction::Allow => "allow",
            RuleAction::Deny => "deny",
            RuleAction::Reject => "reject",
            RuleAction::Limit => "limit",
        }
    }

    pub fn libelle(&self) -> &'static str {
        match self {
            RuleAction::Allow => "Autoriser",
            RuleAction::Deny => "Bloquer",
            RuleAction::Reject => "Rejeter",
            RuleAction::Limit => "Limiter",
        }
    }
}

/// Sens d'une règle (« ALLOW IN », « ALLOW OUT »…).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleDir {
    In,
    Out,
    Fwd,
}

impl RuleDir {
    pub fn libelle(&self) -> &'static str {
        match self {
            RuleDir::In => "Entrée",
            RuleDir::Out => "Sortie",
            RuleDir::Fwd => "Routage",
        }
    }
}

/// Une règle UFW, telle que parsée depuis `ufw status numbered`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub number: u32,
    pub action: RuleAction,
    pub dir: RuleDir,
    /// Colonne « To » : port/proto, profil d'application ou « Anywhere ».
    pub to: String,
    /// Colonne « From » : « Anywhere » ou une adresse/CIDR.
    pub from: String,
    pub comment: Option<String>,
    pub ipv6: bool,
}

/// Protocole d'une nouvelle règle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Proto {
    Tcp,
    Udp,
    Any,
}

/// Sens d'une nouvelle règle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NewRuleDir {
    In,
    Out,
}

/// Nouvelle règle à créer. Les champs sont validés par l'interface avant
/// d'arriver ici ; le cœur les convertit en arguments `ufw`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewRule {
    pub action: RuleAction,
    pub dir: NewRuleDir,
    /// Port, plage (« 8000:8010 »), liste (« 80,443 ») ou nom de service.
    /// `None` si un profil d'application est utilisé.
    pub port: Option<String>,
    pub proto: Proto,
    /// Adresse ou CIDR source ; `None` = partout.
    pub from_addr: Option<String>,
    /// Adresse ou CIDR destination ; `None` = partout.
    pub to_addr: Option<String>,
    /// Profil d'application (`ufw app list`) ; prioritaire sur le port.
    pub app_profile: Option<String>,
    /// Position d'insertion (1 = en tête) ; `None` = à la fin.
    pub insert_at: Option<u32>,
    /// Commentaire libre (affiché dans la table).
    pub comment: Option<String>,
}

/// État global de Fail2Ban (`fail2ban-client status`).
#[derive(Debug, Clone, Default)]
pub struct F2bGlobal {
    pub jail_count: u32,
    pub jails: Vec<String>,
}

/// Détail d'une prison (`fail2ban-client status <jail>` + paramètres).
#[derive(Debug, Clone, Default)]
pub struct JailInfo {
    pub name: String,
    pub currently_failed: Option<u64>,
    pub total_failed: Option<u64>,
    pub currently_banned: Option<u64>,
    pub total_banned: Option<u64>,
    /// Fichiers surveillés ou filtres systemd.
    pub file_list: Option<String>,
    pub banned_ips: Vec<String>,
    pub bantime: Option<String>,
    pub findtime: Option<String>,
    pub maxretry: Option<String>,
}

/// Source des journaux système lisibles par l'application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogSource {
    Fail2ban,
    UfwBlocks,
}
