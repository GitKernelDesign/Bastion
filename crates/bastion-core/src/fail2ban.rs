//! Contrôleur Fail2Ban : commandes et analyse des sorties de
//! `fail2ban-client`.
//!
//! Les parseurs sont volontairement tolérants : la mise en forme de
//! `fail2ban-client status` varie entre versions (tabulations, alignements,
//! listes à la ligne). On extrait des paires « clé : valeur » plutôt qu'une
//! grille figée.

use crate::error::CoreError;
use crate::exec::{CommandSpec, Program};
use crate::model::{F2bGlobal, JailInfo, LogSource};
use crate::transport::{exec_checked, exec_lax, Transport};

/// Parse la sortie de `fail2ban-client status` (état global).
pub fn parse_global(raw: &str) -> F2bGlobal {
    let mut g = F2bGlobal::default();
    for line in raw.lines() {
        if let Some((k, v)) = split_key_value(line) {
            if k == "Number of jail" {
                g.jail_count = v.trim().parse().unwrap_or(0);
            } else if k == "Jail list" {
                g.jails = v
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect();
            }
        }
    }
    if g.jail_count == 0 {
        g.jail_count = g.jails.len() as u32;
    }
    g
}

/// Extrait « Clé : valeur » d'une ligne décorée (`|-`, `` ` ``, `|`).
/// Retourne (clé, valeur) normalisées, sans les décorations.
fn split_key_value(line: &str) -> Option<(String, String)> {
    let mut s = line.trim();
    while let Some(rest) = s
        .strip_prefix("|-")
        .or_else(|| s.strip_prefix("`-"))
        .or_else(|| s.strip_prefix('|'))
        .or_else(|| s.strip_prefix('`'))
    {
        s = rest.trim_start();
    }
    let s = s.trim();
    let (k, v) = s.split_once(':')?;
    Some((k.trim().to_string(), v.trim().to_string()))
}

/// Parse la sortie de `fail2ban-client status <jail>`, en gérant les listes
/// d'IP bannies qui continuent sur plusieurs lignes (y compris IPv6, qui
/// contiennent des « : » et ne doivent pas être prises pour des clés).
pub fn parse_jail_status(name: &str, raw: &str) -> JailInfo {
    let mut fields: Vec<(String, String)> = Vec::new();
    let mut continuation = false;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Les nouvelles paires clé/valeur arrivent décorées (« |- », « `- », « | »).
        let decorated = trimmed.starts_with('|') || trimmed.starts_with('`');
        if decorated {
            if let Some((k, v)) = split_key_value(line) {
                continuation = k.eq_ignore_ascii_case("Banned IP list") && v.is_empty();
                fields.push((k, v));
                continue;
            }
        }
        // Ligne de suite d'une liste (IP bannies) : on accumule dans la
        // dernière valeur tant que le mode continuation est actif.
        if continuation {
            if let Some((_, last_v)) = fields.last_mut() {
                last_v.push(' ');
                last_v.push_str(trimmed);
            }
        }
    }

    let get = |key: &str| -> Option<String> {
        fields
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.clone())
    };

    let banned_raw = get("Banned IP list").unwrap_or_default();
    let banned_ips: Vec<String> = banned_raw
        .split_whitespace()
        .map(str::to_string)
        .filter(|s| !s.is_empty())
        .collect();

    JailInfo {
        name: name.to_string(),
        currently_failed: get("Currently failed").and_then(|v| parse_u64(&v)),
        total_failed: get("Total failed").and_then(|v| parse_u64(&v)),
        currently_banned: get("Currently banned").and_then(|v| parse_u64(&v)),
        total_banned: get("Total banned").and_then(|v| parse_u64(&v)),
        file_list: get("File list")
            .or_else(|| get("Journal matches"))
            .or_else(|| get("Files")),
        banned_ips,
        bantime: None,
        findtime: None,
        maxretry: None,
    }
}

fn parse_u64(s: &str) -> Option<u64> {
    s.trim().parse().ok()
}

/// Parse la sortie de `fail2ban-client get <jail> bannedips` (une IP/ligne).
pub fn parse_bannedips(raw: &str) -> Vec<String> {
    raw.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

/// Humanise une durée en secondes (« 600 » → « 10 min »).
pub fn humanize_duration(raw: &str) -> String {
    let Some(secs) = raw.trim().parse::<u64>().ok() else {
        return raw.trim().to_string();
    };
    if secs == 0 {
        return "0 s".into();
    }
    let d = secs / 86400;
    let h = (secs % 86400) / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    let mut out = String::new();
    if d > 0 {
        out.push_str(&format!("{d} j "));
    }
    if h > 0 {
        out.push_str(&format!("{h} h "));
    }
    if m > 0 {
        out.push_str(&format!("{m} min "));
    }
    if s > 0 && d == 0 {
        out.push_str(&format!("{s} s"));
    }
    out.trim().to_string()
}

pub async fn global_status(t: &dyn Transport) -> Result<F2bGlobal, CoreError> {
    let raw = exec_checked(t, &CommandSpec::new(Program::Fail2banClient, &["status"])).await?;
    Ok(parse_global(&raw))
}

pub async fn jail_status(t: &dyn Transport, jail: &str) -> Result<JailInfo, CoreError> {
    let raw = exec_checked(
        t,
        &CommandSpec::new(Program::Fail2banClient, &["status", jail]),
    )
    .await?;
    let mut info = parse_jail_status(jail, &raw);

    // NB : « get <jail> bannedips » n'existe pas en fail2ban 1.x
    // (« Invalid command ») — la liste des IP vient de l'analyse du status.

    // Paramètres : tolère l'absence individuelle.
    for (param, slot) in [
        ("bantime", &mut info.bantime),
        ("findtime", &mut info.findtime),
        ("maxretry", &mut info.maxretry),
    ] {
        if let Ok(out) = exec_lax(
            t,
            &CommandSpec::new(Program::Fail2banClient, &["get", jail, param]),
        )
        .await
        {
            if out.success() {
                let v = out.stdout.trim().to_string();
                if !v.is_empty() {
                    *slot = Some(v);
                }
            }
        }
    }
    Ok(info)
}

pub async fn ban_ip(t: &dyn Transport, jail: &str, ip: &str) -> Result<(), CoreError> {
    exec_checked(
        t,
        &CommandSpec::new(Program::Fail2banClient, &["set", jail, "banip", ip]),
    )
    .await?;
    Ok(())
}

pub async fn unban_ip(t: &dyn Transport, jail: &str, ip: &str) -> Result<(), CoreError> {
    exec_checked(
        t,
        &CommandSpec::new(Program::Fail2banClient, &["set", jail, "unbanip", ip]),
    )
    .await?;
    Ok(())
}

/// Toutes les IP bannies, toutes prisons confondues (`fail2ban-client banned`).
pub async fn banned_all(t: &dyn Transport) -> Result<Vec<(String, Vec<String>)>, CoreError> {
    let out = exec_lax(
        t,
        &CommandSpec::new(Program::Fail2banClient, &["banned"]),
    )
    .await?;
    if !out.success() {
        return Err(CoreError::CommandFailed {
            program: "fail2ban-client".into(),
            code: out.code,
            stderr: out.stderr.trim().to_string(),
        });
    }
    Ok(parse_banned_all(&out.stdout))
}

/// Sortie réelle de `fail2ban-client banned` (fail2ban 1.x) : représentation
/// Python du dict, p. ex. `[{'sshd': ['1.2.3.4']}, {'nginx': []}]`.
/// (Les anciennes versions imprimaient « Jail name: … / Banned IPs: … » —
/// les deux formats sont acceptés.)
pub fn parse_banned_all(raw: &str) -> Vec<(String, Vec<String>)> {
    let mut result = Vec::new();
    let text = raw;

    // Format 1.x : paires 'jail': [ips] dans une représentation Python.
    let mut i = 0;
    let bytes = text.as_bytes();
    while i < bytes.len() {
        // cherche une apostrophe ouvrante suivie du nom puis « ' » puis « : » puis « [ »
        if bytes[i] == b'\'' {
            if let Some(close) = text[i + 1..].find('\'') {
                let name = &text[i + 1..i + 1 + close];
                let after = &text[i + 1 + close..];
                if after.starts_with('\'') && after[1..].trim_start().starts_with(':') {
                    // cherche le crochet de liste
                    if let Some(bracket) = after[1..].find('[') {
                        let list_start = i + 1 + close + 1 + bracket + 1;
                        if let Some(list_end) = text[list_start..].find(']') {
                            let ips_raw = &text[list_start..list_start + list_end];
                            let ips: Vec<String> = ips_raw
                                .split(',')
                                .map(|s| s.trim().trim_matches('\'').trim_matches('"').to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                            result.push((name.to_string(), ips));
                            i = list_start + list_end;
                        }
                    }
                }
            }
        }
        i += 1;
    }

    if result.is_empty() {
        // Format historique : « Jail name: … / Banned IPs: … ».
        let mut current: Option<(String, Vec<String>)> = None;
        for line in text.lines() {
            let line = line.trim();
            if let Some(jail) = line.strip_prefix("Jail name:") {
                if let Some(c) = current.take() {
                    result.push(c);
                }
                current = Some((jail.trim().to_string(), Vec::new()));
            } else if let Some(ips) = line.strip_prefix("Banned IPs:") {
                if let Some((_, list)) = current.as_mut() {
                    *list = ips
                        .split_whitespace()
                        .map(str::to_string)
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            }
        }
        if let Some(c) = current.take() {
            result.push(c);
        }
    }
    result
}

pub async fn reload(t: &dyn Transport, jail: Option<&str>) -> Result<(), CoreError> {
    let spec = match jail {
        Some(j) => CommandSpec::new(Program::Fail2banClient, &["reload", j]),
        None => CommandSpec::new(Program::Fail2banClient, &["reload"]),
    };
    exec_checked(t, &spec).await?;
    Ok(())
}

/// État du service systemd fail2ban : « active », « inactive », « failed »…
pub async fn service_state(t: &dyn Transport) -> Result<String, CoreError> {
    let out = exec_lax(
        t,
        &CommandSpec::new(Program::Systemctl, &["is-active", "fail2ban"]),
    )
    .await?;
    Ok(out.stdout.trim().to_string())
}

pub async fn service_action(t: &dyn Transport, action: &str) -> Result<(), CoreError> {
    let out = exec_lax(
        t,
        &CommandSpec::new(Program::Systemctl, &[action, "fail2ban"]),
    )
    .await?;
    if out.success() {
        Ok(())
    } else {
        Err(CoreError::CommandFailed {
            program: "systemctl".into(),
            code: out.code,
            stderr: out.stderr.trim().to_string(),
        })
    }
}

/// Journaux système : `journalctl -u fail2ban` ou blocages UFW (`-k --grep`).
pub async fn system_logs(t: &dyn Transport, which: LogSource, lines: u32) -> Result<String, CoreError> {
    let lines_s = lines.to_string();
    let spec = match which {
        LogSource::Fail2ban => {
            CommandSpec::new(Program::Journalctl, &["-u", "fail2ban", "-n", &lines_s, "--no-pager"])
        }
        LogSource::UfwBlocks => CommandSpec::new(
            Program::Journalctl,
            &["-k", "--grep", "UFW", "-n", &lines_s, "--no-pager"],
        ),
    };
    exec_checked(t, &spec).await
}

#[cfg(test)]
mod tests {
    use super::*;

    const GLOBAL: &str = "\
Status
|- Number of jail:\t3
`- Jail list:\tsshd, nginx-badbots, dovecot
";

    const JAIL: &str = "\
Status for the jail: sshd
|- Filter
|  |- Currently failed:\t\t3
|  |- Total failed:\t\t42
|  |- Journal matches:\t_SYSTEMD_UNIT=sshd.service + _COMM=sshd
`- Actions
   |- Currently banned:\t\t2
   |- Total banned:\t\t12
   `- Banned IP list:\t203.0.113.40 198.51.100.7
";

    const JAIL_FILELIST: &str = "\
Status for the jail: nginx-badbots
|- Filter
|  |- Currently failed:\t0
|  |- Total failed:\t0
|  |- File list:\t/var/log/nginx/access.log
`- Actions
   |- Currently banned:\t1
   |- Total banned:\t1
   `- Banned IP list:\t203.0.113.99
";

    #[test]
    fn parse_global_status() {
        let g = parse_global(GLOBAL);
        assert_eq!(g.jail_count, 3);
        assert_eq!(g.jails.len(), 3);
        assert_eq!(g.jails[0], "sshd");
        assert_eq!(g.jails[2], "dovecot");
    }

    #[test]
    fn parse_jail_avec_journal_matches() {
        let j = parse_jail_status("sshd", JAIL);
        assert_eq!(j.currently_failed, Some(3));
        assert_eq!(j.total_failed, Some(42));
        assert_eq!(j.currently_banned, Some(2));
        assert_eq!(j.total_banned, Some(12));
        assert_eq!(
            j.file_list.as_deref(),
            Some("_SYSTEMD_UNIT=sshd.service + _COMM=sshd")
        );
        assert_eq!(j.banned_ips, vec!["203.0.113.40", "198.51.100.7"]);
    }

    #[test]
    fn parse_jail_avec_file_list() {
        let j = parse_jail_status("nginx-badbots", JAIL_FILELIST);
        assert_eq!(j.file_list.as_deref(), Some("/var/log/nginx/access.log"));
        assert_eq!(j.banned_ips, vec!["203.0.113.99"]);
    }

    #[test]
    fn parse_bannedips_une_par_ligne() {
        let raw = "1.2.3.4\n5.6.7.8\n";
        assert_eq!(parse_bannedips(raw), vec!["1.2.3.4", "5.6.7.8"]);
    }

    #[test]
    fn parse_banned_all_par_prison() {
        let raw = "\
Jail name:\tsshd
Banned IPs:\t1.2.3.4 5.6.7.8
Jail name:\tnginx
Banned IPs:\t9.9.9.9
";
        let r = parse_banned_all(raw);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].0, "sshd");
        assert_eq!(r[0].1, vec!["1.2.3.4", "5.6.7.8"]);
        assert_eq!(r[1].1, vec!["9.9.9.9"]);
    }

    #[test]
    fn humanise_durees() {
        assert_eq!(humanize_duration("600"), "10 min");
        assert_eq!(humanize_duration("45"), "45 s");
        assert_eq!(humanize_duration("3600"), "1 h");
        assert_eq!(humanize_duration("86400"), "1 j");
        assert_eq!(humanize_duration("90000"), "1 j 1 h");
        assert_eq!(humanize_duration("1d"), "1d"); // formats non numériques passés tels quels
    }
}
