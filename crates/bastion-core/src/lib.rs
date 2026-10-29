//! Cœur applicatif de Bastion.
//!
//! Ce crate ne dépend d'aucun toolkit graphique : il définit les modèles de
//! données, les parseurs des sorties d'`ufw` et de `fail2ban-client`, la liste
//! blanche des commandes exécutables, et les transports (exécution locale
//! directe, helper privilégié via polkit, SSH via `russh`).

pub mod error;
pub mod exec;
pub mod fail2ban;
pub mod helper;
pub mod local;
pub mod model;
pub mod op;
pub mod ssh;
pub mod transport;
pub mod ufw;

pub use error::CoreError;
pub use op::{Op, OpResult};
pub use transport::{ExecOutput, Transport};