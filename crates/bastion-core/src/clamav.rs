//! Contrôleur ClamAV : services systemd, bases de signatures, analyses.
//! Toutes les commandes passent par la liste blanche de [`crate::exec`].

use crate::error::CoreError;
use crate::exec::{CommandSpec, Program};
use crate::model::{ClamDb, ClamLogSource, ClamScanResult};
use crate::op::ServiceAction;
use crate::transport::{exec_lax, Transport};

/// État systemd de clamav-daemon et clamav-freshclam.
/// Retourne (démon, freshclam) : « active », « inactive », « not-found »…
pub async fn services_state(t: &dyn Transport) -> Result<(String, String), CoreError> {
    let daemon = exec_lax(
        t,
        &CommandSpec::new(Program::Systemctl, &["is-active", "clamav-daemon"]),
    )
    .await?;
    let freshclam = exec_lax(
        t,
        &CommandSpec::new(Program::Systemctl, &["is-active", "clamav-freshclam"]),
    )
    .await?;
    Ok((
        daemon.stdout.trim().to_string(),
        freshclam.stdout.trim().to_string(),
    ))
}

/// Action systemd sur une unité ClamAV (start/stop/restart…).
pub async fn service_action(t: &dyn Transport, unit: &str, action: &str) -> Result<(), CoreError> {
    let out = exec_lax(
        t,
        &CommandSpec::new(Program::Systemctl, &[action, unit]),
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

/// Parse « ClamAV 1.4.6/28169/Fri Oct 30 07:26:48 2026 ».
pub fn parse_version(raw: &str) -> Option<ClamDb> {
    let raw = raw.trim();
    let rest = raw.strip_prefix("ClamAV ")?;
    let (engine, rest) = rest.split_once('/')?;
    let (sigs, db_date) = rest.split_once('/')?;
    Some(ClamDb {
        engine: engine.to_string(),
        sigs: sigs.trim().parse().ok()?,
        db_date: db_date.trim().to_string(),
    })
}

/// Bases de signatures via `clamscan --version`.
pub async fn db_info(t: &dyn Transport) -> Result<ClamDb, CoreError> {
    let out = exec_lax(t, &CommandSpec::new(Program::Clamscan, &["--version"])).await?;
    parse_version(&out.stdout).ok_or(CoreError::ParseFailed {
        program: "clamscan".into(),
        detail: format!("version illisible : « {} »", out.stdout.trim()),
    })
}

/// Parse les lignes « chemin: Signature VIRUS FOUND » d'une analyse.
pub fn parse_scan(raw: &str) -> ClamScanResult {
    let mut infected = Vec::new();
    for line in raw.lines() {
        if let Some(hit) = line.strip_suffix(" FOUND") {
            if let Some((path, virus)) = hit.split_once(": ") {
                infected.push((path.trim().to_string(), virus.trim().to_string()));
            }
        }
    }
    ClamScanResult { infected }
}

/// Analyse un chemin : via le démon (clamdscan, rapide) ou en autonome
/// (clamscan, plus lent mais sans démon). Lecture seule, jamais de
/// suppression ni de quarantaine automatique.
pub async fn scan(
    t: &dyn Transport,
    path: &str,
    use_daemon: bool,
) -> Result<ClamScanResult, CoreError> {
    let out = if use_daemon {
        exec_lax(
            t,
            &CommandSpec::new(
                Program::Clamdscan,
                &["--no-summary", "--multiscan", "--fdpass", path],
            ),
        )
        .await?
    } else {
        exec_lax(
            t,
            &CommandSpec::new(Program::Clamscan, &["-i", "-r", "--no-summary", path]),
        )
        .await?
    };
    // clamscan/clamdscan : 0 = rien trouvé, 1 = infecté, 2 = erreur.
    if out.code > 1 {
        return Err(CoreError::CommandFailed {
            program: if use_daemon { "clamdscan" } else { "clamscan" }.into(),
            code: out.code,
            stderr: out.stderr.trim().to_string(),
        });
    }
    Ok(parse_scan(&out.stdout))
}

/// Journaux systemd d'une unité ClamAV.
pub async fn logs(t: &dyn Transport, which: ClamLogSource, lines: u32) -> Result<String, CoreError> {
    let unit = match which {
        ClamLogSource::Daemon => "clamav-daemon",
        ClamLogSource::Freshclam => "clamav-freshclam",
    };
    let n = lines.to_string();
    let out = exec_lax(
        t,
        &CommandSpec::new(Program::Journalctl, &["-u", unit, "-n", &n, "--no-pager"]),
    )
    .await?;
    Ok(out.stdout)
}

/// Conversion de l'action de service en nom systemctl (exposé pour l'UI).
pub fn action_name(action: ServiceAction) -> &'static str {
    action.systemctl_name()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_version_reelle() {
        let db = parse_version("ClamAV 1.4.6/28169/Fri Oct 30 07:26:48 2026").unwrap();
        assert_eq!(db.engine, "1.4.6");
        assert_eq!(db.sigs, 28169);
        assert_eq!(db.db_date, "Fri Oct 30 07:26:48 2026");
    }

    #[test]
    fn parse_version_invalide() {
        assert!(parse_version("n'importe quoi").is_none());
        assert!(parse_version("ClamAV 1.4.6/abc/d").is_none());
    }

    #[test]
    fn parse_scan_sortie() {
        let raw = "/tmp/eicar.txt: Eicar-Test-Signature FOUND\n/tmp/ok.txt: OK\n";
        let r = parse_scan(raw);
        assert_eq!(r.infected.len(), 1);
        assert_eq!(r.infected[0].0, "/tmp/eicar.txt");
        assert_eq!(r.infected[0].1, "Eicar-Test-Signature");
    }

    #[test]
    fn parse_scan_vide() {
        let r = parse_scan("");
        assert!(r.infected.is_empty());
    }

    /// Intégration réelle (ignorée par défaut) : exige ClamAV local et
    /// /tmp/clamtest avec le fichier EICAR de test.
    /// Lancer : cargo test -p bastion-core --ignored -- --nocapture
    #[tokio::test]
    #[ignore = "nécessite ClamAV installé + /tmp/clamtest (EICAR)"]
    async fn integration_reelle_scan_et_version() {
        let t = crate::local::LocalRoot;
        let db = db_info(&t).await.expect("clamscan --version");
        assert!(!db.engine.is_empty(), "version moteur : {db:?}");
        assert!(db.sigs > 0, "signatures : {db:?}");

        let (daemon, freshclam) = services_state(&t).await.expect("systemctl is-active");
        println!("daemon={daemon:?} freshclam={freshclam:?}");
        assert!(daemon == "active" || daemon == "inactive", "état inattendu : {daemon}");

        let r = scan(&t, "/tmp/clamtest", daemon == "active").await.expect("scan");
        assert_eq!(r.infected.len(), 1, "EICAR attendu : {:?}", r.infected);
        assert!(r.infected[0].1.contains("Eicar"));
    }
}
