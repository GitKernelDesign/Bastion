//! Abstraction de transport : exécuter une commande sur la cible, quelle que
//! soit la manière d'y accéder (local root, helper polkit, SSH).

use crate::error::CoreError;
use crate::exec::{ensure_whitelisted, CommandSpec};
use async_trait::async_trait;

/// Résultat d'une commande système.
#[derive(Debug, Clone, Default)]
pub struct ExecOutput {
    pub stdout: String,
    pub stderr: String,
    pub code: i32,
}

impl ExecOutput {
    pub fn success(&self) -> bool {
        self.code == 0
    }
}

/// Un transport est capable d'exécuter des [`CommandSpec`] sur une cible.
#[async_trait]
pub trait Transport: Send + Sync {
    async fn exec(&self, spec: &CommandSpec) -> Result<ExecOutput, CoreError>;

    /// Libère la connexion (déconnexion SSH, fermeture socket…).
    async fn shutdown(&self) {}

    /// Description affichée dans la barre supérieure.
    fn describe(&self) -> String;
}

/// Exécute une spec et exige un code nul. En cas d'échec, le message système
/// est complété d'une piste actionnable quand la cause est identifiable.
pub async fn exec_checked(t: &dyn Transport, spec: &CommandSpec) -> Result<String, CoreError> {
    ensure_whitelisted(spec)?;
    let out = t.exec(spec).await?;
    if out.success() {
        return Ok(out.stdout);
    }
    let mut stderr = out.stderr.trim().to_string();
    if stderr.is_empty() {
        stderr = out.stdout.trim().to_string();
    }
    // Indices ergonomiques : les causes les plus fréquentes reçoivent une
    // explication en clair, la commande exacte restant dans le Journal.
    if stderr.contains("You need to be root") {
        stderr.push_str(
            "\n→ La cible exige les droits root : en SSH, activez « Exécuter via sudo » (avec le mot de passe sudo si nécessaire) ou connectez-vous en root.",
        );
    } else if stderr.contains("a password is required")
        || stderr.contains("sudo: no tty")
        || stderr.contains("no password was provided")
    {
        stderr.push_str(
            "\n→ Le sudo distant exige un mot de passe : renseignez « Mot de passe sudo » sur l'écran de connexion.",
        );
    } else if stderr.contains("could not be found") || stderr.contains("command not found") {
        stderr.push_str(
            "\n→ L'outil semble absent de la cible : vérifiez qu'ufw/fail2ban y sont installés.",
        );
    }
    Err(CoreError::CommandFailed {
        program: spec.program.name().to_string(),
        code: out.code,
        stderr,
    })
}

/// Comme [`exec_checked`] mais tolère un code non nul (sorties d'information).
pub async fn exec_lax(t: &dyn Transport, spec: &CommandSpec) -> Result<ExecOutput, CoreError> {
    ensure_whitelisted(spec)?;
    t.exec(spec).await
}
