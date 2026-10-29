//! Exécution locale directe (l'application tourne elle-même en root).

use crate::error::CoreError;
use crate::exec::{resolve_local, CommandSpec};
use crate::transport::{ExecOutput, Transport};
use async_trait::async_trait;

/// Transport root direct : utilisé quand l'interface est lancée en root,
/// ou quand le helper a déjà été escaladé.
pub struct LocalRoot;

#[async_trait]
impl Transport for LocalRoot {
    async fn exec(&self, spec: &CommandSpec) -> Result<ExecOutput, CoreError> {
        let path = resolve_local(spec.program).ok_or_else(|| CoreError::ProgramNotFound {
            program: spec.program.name().to_string(),
        })?;
        let out = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            tokio::process::Command::new(&path).args(&spec.args).output(),
        )
        .await
        .map_err(|_| CoreError::Other(format!("{} : délai dépassé", spec.program)))?
        .map_err(|e| CoreError::Other(format!("exécution de {} : {e}", spec.program)))?;

        Ok(ExecOutput {
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
            code: out.status.code().unwrap_or(-1),
        })
    }

    fn describe(&self) -> String {
        "Local (root)".to_string()
    }
}
