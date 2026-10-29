//! Spécification des commandes système et liste blanche.
//!
//! Toute commande exécutée par Bastion passe par une [`CommandSpec`], validée
//! deux fois : côté interface avant l'envoi, et côté helper root avant
//! l'exécution (défense en profondeur). Les programmes sont invoqués par
//! argv direct en local (jamais via un shell) ; en SSH, chaque argument est
//! échappé par [`quote_shell`].

use crate::error::CoreError;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Programmes autorisés. C'est la *seule* surface d'attaque offerte par le
/// helper root : rien d'autre ne peut être exécuté.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Program {
    Ufw,
    Fail2banClient,
    Systemctl,
    Journalctl,
}

impl Program {
    pub fn name(&self) -> &'static str {
        match self {
            Program::Ufw => "ufw",
            Program::Fail2banClient => "fail2ban-client",
            Program::Systemctl => "systemctl",
            Program::Journalctl => "journalctl",
        }
    }

    pub fn from_name(s: &str) -> Option<Program> {
        match s {
            "ufw" => Some(Program::Ufw),
            "fail2ban-client" => Some(Program::Fail2banClient),
            "systemctl" => Some(Program::Systemctl),
            "journalctl" => Some(Program::Journalctl),
            _ => None,
        }
    }
}

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Une commande à exécuter sur la cible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSpec {
    pub program: Program,
    pub args: Vec<String>,
}

impl CommandSpec {
    pub fn new(program: Program, args: &[&str]) -> CommandSpec {
        CommandSpec {
            program,
            args: args.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Représentation lisible (journal, aperçu dans l'interface).
    pub fn display(&self) -> String {
        let mut s = String::from(self.program.name());
        for a in &self.args {
            s.push(' ');
            s.push_str(a);
        }
        s
    }

    /// Commande shell exacte envoyée en SSH (arguments échappés).
    pub fn to_shell(&self) -> String {
        let mut s = String::from(self.program.name());
        for a in &self.args {
            s.push(' ');
            s.push_str(&quote_shell(a));
        }
        s
    }
}

/// Échappement POSIX : tout est entre apostrophes, les apostrophes internes
/// sont refermées-échappées-rouvertes. Aucun contenu ne peut sortir du
/// délimiteur, l'injection shell est impossible.
pub fn quote_shell(arg: &str) -> String {
    if !arg.is_empty()
        && arg
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./:=@+%".contains(c))
    {
        arg.to_string()
    } else {
        format!("'{}'", arg.replace('\'', "'\\''"))
    }
}

/// Garde-fous génériques : pas d'argument vide, interdiction des caractères
/// de contrôle, taille raisonnable.
///
/// Note sécurité : l'exécution locale se fait par argv direct (jamais de
/// shell) et le transport SSH échappe chaque argument via [`quote_shell`]
/// (testée unitairement) — les métacaractères ne sont donc pas dangereux
/// en soi ; les rejeter casserait des commentaires légitimes (« SMTP ; prod »).
fn generic_ok(args: &[String]) -> Result<(), String> {
    if args.len() > 48 {
        return Err("trop d'arguments".into());
    }
    for a in args {
        if a.is_empty() {
            return Err("argument vide".into());
        }
        if a.len() > 512 {
            return Err("argument trop long".into());
        }
        if a.chars().any(|c| c.is_control()) {
            return Err("caractère de contrôle interdit".into());
        }
    }
    Ok(())
}

/// Validation complète d'une spec par la liste blanche.
/// Retourne une description lisible de la raison du refus.
pub fn validate(spec: &CommandSpec) -> Result<(), String> {
    generic_ok(&spec.args)?;

    match spec.program {
        Program::Ufw => validate_ufw(&spec.args),
        Program::Fail2banClient => validate_f2b(&spec.args),
        Program::Systemctl => validate_systemctl(&spec.args),
        Program::Journalctl => validate_journalctl(&spec.args),
    }
}

fn validate_ufw(args: &[String]) -> Result<(), String> {
    let first = args.first().map(|s| s.as_str()).unwrap_or_default();
    match first {
        "--version" | "version" => Ok(()),
        "status" => {
            if args.len() == 1 {
                Ok(())
            } else if args.len() == 2
                && matches!(args[1].as_str(), "verbose" | "numbered")
            {
                Ok(())
            } else {
                Err("forme « status [verbose|numbered] » attendue".into())
            }
        }
        "enable" | "disable" | "reload" | "reset" if args.len() == 1 => Ok(()),
        "default" if args.len() == 3 => {
            let policy = args[1].as_str();
            let dir = args[2].as_str();
            let policy_ok = matches!(policy, "allow" | "deny" | "reject");
            let dir_ok = matches!(dir, "incoming" | "outgoing" | "routed");
            if policy_ok && dir_ok {
                Ok(())
            } else {
                Err("forme « default <allow|deny|reject> <incoming|outgoing|routed> » attendue".into())
            }
        }
        "logging" => {
            let lvl = args.get(1).map(|s| s.as_str()).unwrap_or_default();
            matches!(lvl, "on" | "off" | "low" | "medium" | "high" | "full")
                .then_some(())
                .ok_or_else(|| "niveau de journalisation inconnu".to_string())
        }
        "app" => match args.get(1).map(|s| s.as_str()) {
            Some("list") | Some("--list-available") if args.len() == 2 => Ok(()),
            _ => Err("sous-commande ufw app non autorisée".into()),
        },
        "show" => match args.get(1).map(|s| s.as_str()) {
            Some("listening") | Some("report") | Some("raw") => Ok(()),
            _ => Err("sous-commande ufw show non autorisée".into()),
        },
        "--force" => {
            // --force delete NUM : utilisé pour la suppression sans prompt.
            if args.len() == 3 && args[1] == "delete" && args[2].parse::<u32>().is_ok() {
                Ok(())
            } else {
                Err("forme « ufw --force delete N » attendue".into())
            }
        }
        "insert" => {
            // insert N allow|deny|reject|limit [in|out] [from A] [to A] [port P] [proto T] [comment C]
            let n = args
                .get(1)
                .and_then(|s| s.parse::<u32>().ok())
                .ok_or_else(|| "numéro d'insertion invalide".to_string())?;
            if n == 0 {
                return Err("numéro d'insertion invalide".into());
            }
            validate_rule_args(&args[2..])
        }
        "allow" | "deny" | "reject" | "limit" => validate_rule_args(args),
        _ => Err(format!("commande ufw non autorisée : « {first} »")),
    }
}

/// Valide la partie « règle » : action [in|out] puis soit un positionnel
/// (port/service/profil, éventuellement suivi de « comment X »), soit des
/// clauses from/to/port/proto/comment.
/// Formes vérifiées empiriquement contre ufw 0.36 :
///  allow 22/tcp · allow out 8080 · allow in from A to B port P proto T
///  allow from A to any port P proto tcp comment 'x' · insert N allow …
fn validate_rule_args(args: &[String]) -> Result<(), String> {
    let mut i = 0;
    let action = args.first().map(|s| s.as_str()).unwrap_or_default();
    if !matches!(action, "allow" | "deny" | "reject" | "limit") {
        return Err(format!("action inconnue « {action} »"));
    }
    i += 1;
    // direction optionnelle (« in » / « out »)
    if let Some(dir) = args.get(i) {
        if matches!(dir.as_str(), "in" | "out") {
            i += 1;
        }
    }

    // Forme 1 : positionnel (port / service / profil d'application).
    if let Some(first) = args.get(i) {
        if !matches!(first.as_str(), "from" | "to" | "port" | "proto" | "comment") {
            i += 1;
            // Commentaire optionnel en clôture.
            if args.get(i).map(|s| s.as_str()) == Some("comment") {
                if args.len() != i + 2 {
                    return Err("« comment » attend exactement une valeur".into());
                }
                i += 2;
            }
            if i != args.len() {
                return Err("arguments superflus après la règle".into());
            }
            return Ok(());
        }
    }

    // Forme 2 : clauses nommées.
    while i < args.len() {
        match args[i].as_str() {
            "from" | "to" | "port" | "proto" | "comment" => {
                i += 1;
                if i >= args.len() {
                    return Err(format!("valeur manquante pour « {} »", args[i - 1]));
                }
                i += 1;
            }
            other => {
                return Err(format!("mot-clé inattendu « {other} » dans la règle"));
            }
        }
    }
    Ok(())
}

fn validate_f2b(args: &[String]) -> Result<(), String> {
    let first = args.first().map(|s| s.as_str()).unwrap_or_default();
    let is_jail = |s: &str| {
        !s.is_empty()
            && s.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
            && s.len() <= 64
    };
    let is_ip = |s: &str| {
        !s.is_empty()
            && s.chars().all(|c| c.is_ascii_alphanumeric() || ".:/".contains(c))
            && s.len() <= 64
    };
    match first {
        "--version" | "ping" => Ok(()),
        "status" => {
            // status | status <jail>
            if args.len() == 1 {
                Ok(())
            } else if args.len() == 2 && is_jail(&args[1]) {
                Ok(())
            } else {
                Err("forme « status [jail] » attendue".into())
            }
        }
        "set" => {
            // set <jail> banip|unbanip <ip>
            if args.len() == 4
                && is_jail(&args[1])
                && matches!(args[2].as_str(), "banip" | "unbanip")
                && is_ip(&args[3])
            {
                Ok(())
            } else {
                Err("forme « set <jail> banip|unbanip <ip> » attendue".into())
            }
        }
        "get" => {
            // get <jail> bannedips|bantime|findtime|maxretry
            if args.len() == 3
                && is_jail(&args[1])
                && matches!(
                    args[2].as_str(),
                    "bannedips" | "bantime" | "findtime" | "maxretry"
                )
            {
                Ok(())
            } else {
                Err("paramètre get non autorisé".into())
            }
        }
        "reload" => {
            if args.len() == 1 || (args.len() == 2 && is_jail(&args[1])) {
                Ok(())
            } else {
                Err("forme « reload [jail] » attendue".into())
            }
        }
        "restart" | "unban" | "banned" => {
            if args.len() <= 2 {
                Ok(())
            } else {
                Err("trop d'arguments".into())
            }
        }
        _ => Err(format!("commande fail2ban-client non autorisée : « {first} »")),
    }
}

fn validate_systemctl(args: &[String]) -> Result<(), String> {
    if args.len() < 2 || args.len() > 3 {
        return Err("systemctl attend 2 à 3 arguments".into());
    }
    let action = args[0].as_str();
    let unit = args.last().unwrap();
    let unit = unit.strip_suffix(".service").unwrap_or(unit);
    if !matches!(unit, "fail2ban" | "ufw") {
        return Err(format!("unité systemd non autorisée : « {unit} »"));
    }
    match action {
        "start" | "stop" | "restart" | "reload" | "reload-or-restart" | "is-active"
        | "is-enabled" | "status" => Ok(()),
        _ => Err(format!("action systemctl non autorisée : « {action} »")),
    }
}

fn validate_journalctl(args: &[String]) -> Result<(), String> {
    let mut i = 0;
    let mut has_unit_or_kernel = false;
    while i < args.len() {
        match args[i].as_str() {
            "--no-pager" | "-r" | "-k" | "-b" | "--utc" => {
                if args[i] == "-k" || args[i] == "-b" {
                    has_unit_or_kernel = true;
                }
                i += 1;
            }
            "-u" => {
                let unit = args.get(i + 1).ok_or("valeur manquante pour -u")?;
                let unit = unit.strip_suffix(".service").unwrap_or(unit);
                if !matches!(unit, "fail2ban" | "ufw") {
                    return Err(format!("unité journalctl non autorisée : « {unit} »"));
                }
                has_unit_or_kernel = true;
                i += 2;
            }
            "-n" => {
                args.get(i + 1)
                    .and_then(|s| s.parse::<u32>().ok())
                    .filter(|n| *n <= 1000)
                    .ok_or("« -n » attend un nombre ≤ 1000")?;
                i += 2;
            }
            "--grep" => {
                args.get(i + 1).ok_or("valeur manquante pour --grep")?;
                i += 2;
            }
            other => return Err(format!("option journalctl non autorisée : « {other} »")),
        }
    }
    if !has_unit_or_kernel {
        return Err("journalctl exige -u <unité> ou -k".into());
    }
    Ok(())
}

/// Résout le chemin d'un programme sur la cible locale (PATH + sbin).
pub fn resolve_local(program: Program) -> Option<std::path::PathBuf> {
    let name = program.name();
    let mut candidates: Vec<std::path::PathBuf> = [
        "/usr/sbin", "/sbin", "/usr/local/sbin", "/usr/bin", "/bin", "/usr/local/bin",
    ]
    .iter()
    .map(|d| std::path::PathBuf::from(d).join(name))
    .collect();
    if let Ok(path) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path) {
            candidates.push(dir.join(name));
        }
    }
    candidates
        .into_iter()
        .find(|p| p.is_file())
        .or_else(|| {
            // Dernier recours : laisser le PATH du processus faire le travail.
            if which_in_path(name).is_some() {
                Some(std::path::PathBuf::from(name))
            } else {
                None
            }
        })
}

fn which_in_path(name: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var("PATH").ok()?;
    std::env::split_paths(&path)
        .map(|d| d.join(name))
        .find(|p| p.is_file())
}

/// Convertit une spec en erreur `NotWhitelisted` si elle est refusée.
pub fn ensure_whitelisted(spec: &CommandSpec) -> Result<(), CoreError> {
    validate(spec).map_err(CoreError::NotWhitelisted)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(program: Program, args: &[&str]) -> CommandSpec {
        CommandSpec::new(program, args)
    }

    #[test]
    fn shell_quoting_bloque_injection() {
        assert_eq!(quote_shell("22/tcp"), "22/tcp");
        assert_eq!(quote_shell("a b"), "'a b'");
        assert_eq!(quote_shell("x'; rm -rf /"), "'x'\\''; rm -rf /'");
        assert_eq!(quote_shell("commentaire à l'arrache"), "'commentaire à l'\\''arrache'");
        // L'apostrophe interne ne peut pas fermer le délimiteur
        assert!(quote_shell("'; id").starts_with('\''));
    }

    #[test]
    fn whitelist_ufw() {
        assert!(validate(&spec(Program::Ufw, &["status", "verbose"])).is_ok());
        assert!(validate(&spec(Program::Ufw, &["allow", "22/tcp"])).is_ok());
        assert!(validate(&spec(Program::Ufw, &["allow", "in", "8080"])).is_ok());
        assert!(
            validate(&spec(Program::Ufw, &["allow", "from", "10.0.0.0/8", "to", "any", "port", "8080", "proto", "tcp", "comment", "web"])).is_ok()
        );
        assert!(validate(&spec(Program::Ufw, &["insert", "1", "allow", "22"])).is_ok());
        assert!(validate(&spec(Program::Ufw, &["--force", "delete", "3"])).is_ok());
        assert!(validate(&spec(Program::Ufw, &["app", "list"])).is_ok());
        assert!(validate(&spec(Program::Ufw, &["show", "report"])).is_ok());
        assert!(validate(&spec(Program::Ufw, &["logging", "medium"])).is_ok());
        // Refus
        assert!(validate(&spec(Program::Ufw, &["allow", "22", ";", "id"])).is_err());
        assert!(validate(&spec(Program::Ufw, &["--force", "delete", "abc"])).is_err());
        assert!(validate(&spec(Program::Ufw, &["version"])).is_ok());
        assert!(validate(&spec(Program::Ufw, &["reset"])).is_ok());
        assert!(validate(&spec(Program::Ufw, &["default", "deny", "incoming"])).is_ok());
        assert!(validate(&spec(Program::Ufw, &["default", "deny", "hostile"])).is_err());
    }

    #[test]
    fn whitelist_f2b() {
        assert!(validate(&spec(Program::Fail2banClient, &["status"])).is_ok());
        assert!(validate(&spec(Program::Fail2banClient, &["status", "sshd"])).is_ok());
        assert!(
            validate(&spec(Program::Fail2banClient, &["set", "sshd", "unbanip", "1.2.3.4"])).is_ok()
        );
        assert!(
            validate(&spec(Program::Fail2banClient, &["get", "sshd", "bantime"])).is_ok()
        );
        assert!(
            validate(&spec(Program::Fail2banClient, &["set", "sshd", "unbanip", "x; id"])).is_err()
        );
        assert!(validate(&spec(Program::Fail2banClient, &["reload", "sshd"])).is_ok());
    }

    #[test]
    fn whitelist_systemctl_journalctl() {
        assert!(validate(&spec(Program::Systemctl, &["restart", "fail2ban"])).is_ok());
        assert!(validate(&spec(Program::Systemctl, &["is-active", "fail2ban.service"])).is_ok());
        assert!(validate(&spec(Program::Systemctl, &["start", "nginx"])).is_err());
        assert!(
            validate(&spec(Program::Journalctl, &["-u", "fail2ban", "-n", "200", "--no-pager"])).is_ok()
        );
        assert!(
            validate(&spec(Program::Journalctl, &["-k", "--grep", "UFW", "-n", "200", "--no-pager"])).is_ok()
        );
        assert!(validate(&spec(Program::Journalctl, &["-n", "200"])).is_err());
        assert!(validate(&spec(Program::Journalctl, &["-u", "sshd", "-n", "5"])).is_err());
    }

    #[test]
    fn display_et_shell() {
        let s = spec(Program::Ufw, &["allow", "8080/tcp"]);
        assert_eq!(s.display(), "ufw allow 8080/tcp");
        assert_eq!(s.to_shell(), "ufw allow 8080/tcp");
        let s = spec(Program::Ufw, &["allow", "Apache Full"]);
        assert_eq!(s.to_shell(), "ufw allow 'Apache Full'");
    }
}
