//! Contrôleur UFW : construction des commandes et analyse des sorties.

use crate::error::CoreError;
use crate::exec::{CommandSpec, Program};
use crate::model::*;
use crate::transport::{exec_checked, Transport};

/// Parse la sortie de `ufw status verbose`.
pub fn parse_status_verbose(raw: &str) -> FirewallStatus {
    let mut st = FirewallStatus {
        raw: raw.to_string(),
        ..Default::default()
    };

    for line in raw.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("Status:") {
            st.active = v.trim().eq_ignore_ascii_case("active");
        } else if let Some(v) = line.strip_prefix("Logging:") {
            let v = v.trim();
            st.logging_on = v.starts_with("on");
            if let Some(level) = v
                .strip_prefix("on")
                .and_then(|s| s.trim().trim_start_matches('(').strip_suffix(')'))
                .or_else(|| v.strip_prefix("on"))
            {
                st.logging_level = LogLevel::from_ufw_name(level.trim());
            }
        } else if let Some(v) = line.strip_prefix("Default:") {
            st.defaults = parse_defaults(v.trim());
        }
    }
    st
}

/// Parse « deny (incoming), allow (outgoing), disabled (routed) ».
fn parse_defaults(v: &str) -> Defaults {
    let mut d = Defaults::default();
    for part in v.split(',') {
        let part = part.trim();
        // Forme : « <policy> (<direction>) » ou « disabled (<direction>) »
        let Some(open) = part.find('(') else { continue };
        let Some(close) = part.find(')') else { continue };
        let value = part[..open].trim().to_ascii_lowercase();
        let dir = part[open + 1..close].trim().to_ascii_lowercase();
        let policy = Policy::from_ufw_name(&value);
        match dir.as_str() {
            "incoming" => d.incoming = policy,
            "outgoing" => d.outgoing = policy,
            "routed" => {
                d.routed = policy;
                d.routed_disabled = value == "disabled";
            }
            _ => {}
        }
    }
    d
}

/// Parse la sortie de `ufw status numbered`.
pub fn parse_rules_numbered(raw: &str) -> Result<Vec<Rule>, CoreError> {
    let mut rules = Vec::new();
    for line in raw.lines() {
        let line = line.trim_end();
        let trimmed = line.trim_start();
        if !trimmed.starts_with('[') {
            continue; // en-têtes « Status », « To Action From », séparateurs…
        }
        let Some(close) = trimmed.find(']') else {
            continue;
        };
        let number: u32 = trimmed[1..close]
            .trim()
            .parse()
            .map_err(|_| CoreError::ParseFailed {
                program: "ufw".into(),
                detail: format!("numéro de règle illisible : « {} »", trimmed),
            })?;
        let rest = &trimmed[close + 1..];

        // Le commentaire éventuel est suffixé par « # ».
        let (main_part, comment) = match rest.find('#') {
            Some(pos) => (
                rest[..pos].trim(),
                Some(rest[pos + 1..].trim().to_string()),
            ),
            None => (rest.trim(), None),
        };

        let tokens: Vec<&str> = main_part.split_whitespace().collect();
        // Cherche le jeton d'action : ALLOW / DENY / REJECT / LIMIT.
        let Some(action_pos) = tokens
            .iter()
            .position(|t| matches!(*t, "ALLOW" | "DENY" | "REJECT" | "LIMIT"))
        else {
            continue; // ligne de règle atypique : ignorée
        };

        let action = match tokens[action_pos] {
            "ALLOW" => RuleAction::Allow,
            "DENY" => RuleAction::Deny,
            "REJECT" => RuleAction::Reject,
            _ => RuleAction::Limit,
        };

        let dir = tokens
            .get(action_pos + 1)
            .filter(|t| matches!(**t, "IN" | "OUT" | "FWD"))
            .map(|t| match *t {
                "IN" => RuleDir::In,
                "OUT" => RuleDir::Out,
                _ => RuleDir::Fwd,
            })
            .unwrap_or(RuleDir::In);

        let after_dir = if tokens
            .get(action_pos + 1)
            .map(|t| matches!(*t, "IN" | "OUT" | "FWD"))
            .unwrap_or(false)
        {
            action_pos + 2
        } else {
            action_pos + 1
        };

        let to = tokens[..action_pos].join(" ");
        let from = tokens[after_dir..].join(" ");
        let ipv6 = to.ends_with("(v6)") || from.ends_with("(v6)");
        let clean = |s: &str| s.strip_suffix(" (v6)").unwrap_or(s).to_string();

        rules.push(Rule {
            number,
            action,
            dir,
            to: clean(&to),
            from: clean(&from),
            comment,
            ipv6,
        });
    }
    Ok(rules)
}

/// Parse la sortie de `ufw app list`.
pub fn parse_app_profiles(raw: &str) -> Vec<String> {
    raw.lines()
        .skip_while(|l| !l.contains("Available applications"))
        .skip(1)
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

/// Valide une adresse source/destination : IPv4/IPv6 ou CIDR.
pub fn is_valid_addr(s: &str) -> bool {
    let s = s.trim();
    if s.is_empty() {
        return false;
    }
    // IPv4 simple
    if s.parse::<std::net::Ipv4Addr>().is_ok() {
        return true;
    }
    // IPv4 CIDR
    if let Some((ip, mask)) = s.split_once('/') {
        if ip.parse::<std::net::Ipv4Addr>().is_ok()
            && mask.parse::<u8>().map(|m| m <= 32).unwrap_or(false)
        {
            return true;
        }
    }
    // IPv6 (éventuellement CIDR)
    let base = s.split('/').next().unwrap_or(s);
    if base.parse::<std::net::Ipv6Addr>().is_ok() {
        return true;
    }
    false
}

/// Valide un port/plage/liste ou nom de service : « 22 », « 8000:8010 »,
/// « 80,443 », « ssh », « 8080/tcp ».
pub fn is_valid_port_spec(s: &str) -> bool {
    let s = s.trim();
    if s.is_empty() || s.len() > 64 {
        return false;
    }
    let spec = s.split('/').next().unwrap_or(s);
    spec.split(',').all(|tok| {
        let tok = tok.trim();
        if tok.is_empty() {
            return false;
        }
        if tok.contains(':') {
            let (a, b) = tok.split_once(':').unwrap();
            let (Ok(a), Ok(b)) = (a.parse::<u16>(), b.parse::<u16>()) else {
                return false;
            };
            a <= b
        } else {
            // Nom de service (alphanumérique) ou numéro de port (≤ 65535).
            if tok.chars().all(|c| c.is_ascii_digit()) {
                if let Ok(p) = tok.parse::<u32>() {
                    return p <= 65535;
                }
                return false;
            }
            tok.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        }
    })
}

/// Construit les arguments `ufw` d'une nouvelle règle.
pub fn build_rule_args(rule: &NewRule) -> Result<Vec<String>, CoreError> {
    let mut args: Vec<String> = Vec::new();
    if let Some(pos) = rule.insert_at {
        if pos == 0 {
            return Err(CoreError::Other(
                "la position d'insertion doit être ≥ 1".into(),
            ));
        }
        args.push("insert".into());
        args.push(pos.to_string());
    }
    args.push(rule.action.ufw_name().to_string());

    // Profil d'application : « allow Apache Full » (entrée par défaut).
    if let Some(profile) = &rule.app_profile {
        if rule.port.is_some() || rule.from_addr.is_some() || rule.to_addr.is_some() {
            return Err(CoreError::Other(
                "un profil d'application ne se combine pas avec port/adresses".into(),
            ));
        }
        if rule.dir == NewRuleDir::Out {
            args.push("out".into());
        }
        args.push(profile.clone());
        if let Some(c) = trimmed_comment(rule) {
            args.push("comment".into());
            args.push(c);
        }
        return Ok(args);
    }

    let port = rule
        .port
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .ok_or_else(|| CoreError::Other("le port est obligatoire".into()))?;
    if !is_valid_port_spec(port) {
        return Err(CoreError::Other(format!("port « {port} » invalide")));
    }
    for (label, addr) in [("source", &rule.from_addr), ("destination", &rule.to_addr)] {
        if let Some(a) = addr.as_deref().map(str::trim).filter(|a| !a.is_empty()) {
            if !is_valid_addr(a) {
                return Err(CoreError::Other(format!(
                    "adresse {label} « {a} » invalide"
                )));
            }
        }
    }
    let from = rule.from_addr.as_deref().map(str::trim).filter(|a| !a.is_empty());
    let to = rule.to_addr.as_deref().map(str::trim).filter(|a| !a.is_empty());
    let comment = trimmed_comment(rule);

    // Forme simple (pas d'adresses, pas de commentaire) : le protocole fait
    // partie du spécificateur de port — « allow 8080/tcp ».
    if from.is_none() && to.is_none() && comment.is_none() {
        let spec = match rule.proto {
            Proto::Tcp => format!("{port}/tcp"),
            Proto::Udp => format!("{port}/udp"),
            Proto::Any => port.to_string(),
        };
        if rule.dir == NewRuleDir::Out {
            args.push("out".into());
        }
        args.push(spec);
        return Ok(args);
    }

    // Forme à clauses (adresses et/ou commentaire) — validée contre ufw 0.36 :
    // « allow [in|out] from A [to B] port P [proto T] [comment 'x'] ».
    if rule.dir == NewRuleDir::Out {
        args.push("out".into());
    }
    args.push("from".into());
    args.push(from.unwrap_or("any").to_string());
    if let Some(t) = to {
        args.push("to".into());
        args.push(t.to_string());
    }
    args.push("port".into());
    args.push(port.to_string());
    match rule.proto {
        Proto::Tcp => {
            args.push("proto".into());
            args.push("tcp".into());
        }
        Proto::Udp => {
            args.push("proto".into());
            args.push("udp".into());
        }
        Proto::Any => {}
    }
    if let Some(c) = comment {
        args.push("comment".into());
        args.push(c);
    }
    Ok(args)
}

fn trimmed_comment(rule: &NewRule) -> Option<String> {
    rule.comment
        .as_deref()
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .map(str::to_string)
}

pub async fn status(t: &dyn Transport) -> Result<FirewallStatus, CoreError> {
    let raw = exec_checked(t, &CommandSpec::new(Program::Ufw, &["status", "verbose"])).await?;
    Ok(parse_status_verbose(&raw))
}

pub async fn rules(t: &dyn Transport) -> Result<Vec<Rule>, CoreError> {
    let raw = exec_checked(t, &CommandSpec::new(Program::Ufw, &["status", "numbered"])).await?;
    parse_rules_numbered(&raw)
}

pub async fn enable(t: &dyn Transport) -> Result<(), CoreError> {
    exec_checked(t, &CommandSpec::new(Program::Ufw, &["enable"])).await?;
    Ok(())
}

pub async fn disable(t: &dyn Transport) -> Result<(), CoreError> {
    exec_checked(t, &CommandSpec::new(Program::Ufw, &["disable"])).await?;
    Ok(())
}

pub async fn reload(t: &dyn Transport) -> Result<(), CoreError> {
    exec_checked(t, &CommandSpec::new(Program::Ufw, &["reload"])).await?;
    Ok(())
}

pub async fn reset(t: &dyn Transport) -> Result<(), CoreError> {
    exec_checked(t, &CommandSpec::new(Program::Ufw, &["reset"])).await?;
    Ok(())
}

pub async fn set_default(t: &dyn Transport, dir: Direction, policy: Policy) -> Result<(), CoreError> {
    exec_checked(
        t,
        &CommandSpec::new(Program::Ufw, &["default", policy.ufw_name(), dir.ufw_key()]),
    )
    .await?;
    Ok(())
}

pub async fn set_logging(t: &dyn Transport, level: LogLevel) -> Result<(), CoreError> {
    exec_checked(
        t,
        &CommandSpec::new(Program::Ufw, &["logging", level.ufw_name()]),
    )
    .await?;
    Ok(())
}

pub async fn add_rule(t: &dyn Transport, rule: &NewRule) -> Result<(), CoreError> {
    let args = build_rule_args(rule)?;
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    exec_checked(t, &CommandSpec::new(Program::Ufw, &refs)).await?;
    Ok(())
}

pub async fn delete_rule(t: &dyn Transport, index: u32) -> Result<(), CoreError> {
    exec_checked(
        t,
        &CommandSpec::new(Program::Ufw, &["--force", "delete", &index.to_string()]),
    )
    .await?;
    Ok(())
}

pub async fn app_profiles(t: &dyn Transport) -> Result<Vec<String>, CoreError> {
    let raw = exec_checked(t, &CommandSpec::new(Program::Ufw, &["app", "list"])).await?;
    Ok(parse_app_profiles(&raw))
}

pub async fn report(t: &dyn Transport) -> Result<String, CoreError> {
    exec_checked(t, &CommandSpec::new(Program::Ufw, &["show", "listening"])).await
}

#[cfg(test)]
mod tests {
    use super::*;

    const VERBOSE_ACTIVE: &str = "\
Status: active
Logging: on (low)
Default: deny (incoming), allow (outgoing), disabled (routed)
New profiles: skip

To                         Action      From
--                         ------      ----
22/tcp                     ALLOW IN    Anywhere
22/tcp (v6)                ALLOW IN    Anywhere (v6)
";

    const NUMBERED: &str = "\
Status: active

     To                         Action      From
--                         ------      ----
[ 1] 22/tcp                     ALLOW IN    Anywhere
[ 2] 22/tcp (v6)                ALLOW IN    Anywhere (v6)
[ 3] Apache Full                ALLOW IN    Anywhere
[ 4] 80,443/tcp                 ALLOW IN    192.168.1.0/24
[ 5] 8080                       DENY OUT    Anywhere (v6)
[ 6] Anywhere                  REJECT IN    203.0.113.40
[ 7] 25/tcp                     LIMIT IN    Anywhere                 # SMTP rate-limite
";

    #[test]
    fn parse_status_actif() {
        let st = parse_status_verbose(VERBOSE_ACTIVE);
        assert!(st.active);
        assert!(st.logging_on);
        assert_eq!(st.logging_level, Some(LogLevel::Low));
        assert_eq!(st.defaults.incoming, Some(Policy::Deny));
        assert_eq!(st.defaults.outgoing, Some(Policy::Allow));
        assert!(st.defaults.routed_disabled);
    }

    #[test]
    fn parse_status_inactif() {
        let st = parse_status_verbose("Status: inactive\n");
        assert!(!st.active);
    }

    #[test]
    fn parse_regles_numerotees() {
        let rules = parse_rules_numbered(NUMBERED).unwrap();
        assert_eq!(rules.len(), 7);

        let r1 = &rules[0];
        assert_eq!(r1.number, 1);
        assert_eq!(r1.action, RuleAction::Allow);
        assert_eq!(r1.dir, RuleDir::In);
        assert_eq!(r1.to, "22/tcp");
        assert_eq!(r1.from, "Anywhere");
        assert!(!r1.ipv6);

        let r2 = &rules[1];
        assert!(r2.ipv6);
        assert_eq!(r2.from, "Anywhere");

        // Profil d'application avec espaces.
        let r3 = &rules[2];
        assert_eq!(r3.to, "Apache Full");

        // Source CIDR.
        let r4 = &rules[3];
        assert_eq!(r4.from, "192.168.1.0/24");
        assert_eq!(r4.to, "80,443/tcp");

        // Sens sortie + v6.
        let r5 = &rules[4];
        assert_eq!(r5.dir, RuleDir::Out);
        assert!(r5.ipv6);

        // « Anywhere » dans la colonne To avec source précise.
        let r6 = &rules[5];
        assert_eq!(r6.to, "Anywhere");
        assert_eq!(r6.from, "203.0.113.40");
        assert_eq!(r6.action, RuleAction::Reject);

        // Commentaire.
        let r7 = &rules[6];
        assert_eq!(r7.action, RuleAction::Limit);
        assert_eq!(r7.comment.as_deref(), Some("SMTP rate-limite"));
    }

    #[test]
    fn parse_profils() {
        let raw = "Available applications:\n  Apache\n  CUPS\n  Postfix\n";
        assert_eq!(
            parse_app_profiles(raw),
            vec!["Apache", "CUPS", "Postfix"]
        );
    }

    #[test]
    fn validation_ports_et_adresses() {
        assert!(is_valid_port_spec("22"));
        assert!(is_valid_port_spec("8000:8010"));
        assert!(is_valid_port_spec("80,443"));
        assert!(is_valid_port_spec("ssh"));
        assert!(!is_valid_port_spec(""));
        assert!(!is_valid_port_spec("22;id"));
        assert!(!is_valid_port_spec("90000"));

        assert!(is_valid_addr("192.168.1.5"));
        assert!(is_valid_addr("10.0.0.0/8"));
        assert!(is_valid_addr("fd00::1"));
        assert!(is_valid_addr("fd00::/16"));
        assert!(!is_valid_addr("partout"));
        assert!(!is_valid_addr("1.2.3.4.5"));
    }

    #[test]
    fn construction_de_regle() {
        let mut rule = NewRule {
            action: RuleAction::Allow,
            dir: NewRuleDir::In,
            port: Some("8080".into()),
            proto: Proto::Tcp,
            from_addr: Some("10.0.0.0/8".into()),
            to_addr: None,
            app_profile: None,
            insert_at: Some(1),
            comment: Some("serveur web".into()),
        };
        let args = build_rule_args(&rule).unwrap();
        assert_eq!(
            args,
            vec![
                "insert", "1", "allow", "from", "10.0.0.0/8", "port", "8080", "proto",
                "tcp", "comment", "serveur web"
            ]
        );

        // Forme simple : proto intégré au port, pas de « in » superflu.
        rule.action = RuleAction::Deny;
        rule.dir = NewRuleDir::Out;
        rule.insert_at = None;
        rule.from_addr = None;
        rule.comment = None;
        let args = build_rule_args(&rule).unwrap();
        assert_eq!(args, vec!["deny", "out", "8080/tcp"]);

        // TCP explicite sans adresses ni commentaire.
        rule.dir = NewRuleDir::In;
        rule.proto = Proto::Tcp;
        let args = build_rule_args(&rule).unwrap();
        assert_eq!(args, vec!["deny", "8080/tcp"]);

        // Commentaire seul → forme à clauses avec « from any ».
        rule.comment = Some("web interne".into());
        let args = build_rule_args(&rule).unwrap();
        assert_eq!(
            args,
            vec!["deny", "from", "any", "port", "8080", "proto", "tcp", "comment", "web interne"]
        );

        // Profil d'application seul.
        let rule = NewRule {
            action: RuleAction::Allow,
            dir: NewRuleDir::In,
            port: None,
            proto: Proto::Any,
            from_addr: None,
            to_addr: None,
            app_profile: Some("Apache Full".into()),
            insert_at: None,
            comment: None,
        };
        let args = build_rule_args(&rule).unwrap();
        assert_eq!(args, vec!["allow", "Apache Full"]);

        // Incohérences refusées.
        let mut bad = rule.clone();
        bad.port = Some("80".into());
        assert!(build_rule_args(&bad).is_err());
    }
}
