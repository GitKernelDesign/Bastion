//! Erreurs du cœur applicatif, formulées pour être affichées telles quelles
//! dans l'interface (ergonomie : l'utilisateur voit la cause réelle).

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("la commande « {program} » est introuvable sur la cible")]
    ProgramNotFound { program: String },

    #[error("{program} a échoué (code {code}) : {stderr}")]
    CommandFailed {
        program: String,
        code: i32,
        stderr: String,
    },

    #[error("impossible d'interpréter la sortie de {program} : {detail}")]
    ParseFailed { program: String, detail: String },

    #[error("non connecté")]
    NotConnected,

    #[error("connexion perdue : {0}")]
    Disconnected(String),

    #[error("commande refusée par la liste blanche : {0}")]
    NotWhitelisted(String),

    #[error("authentification SSH refusée pour « {user} »")]
    SshAuthFailed { user: String },

    #[error("clé du serveur refusée")]
    HostKeyRejected,

    #[error("{0}")]
    Other(String),
}

impl CoreError {
    /// Vrai si l'erreur traduit une coupure de la connexion (le worker peut
    /// alors repasser l'état global à « déconnecté »).
    pub fn is_connection_loss(&self) -> bool {
        matches!(
            self,
            CoreError::Disconnected(_) | CoreError::SshAuthFailed { .. }
        )
    }
}
