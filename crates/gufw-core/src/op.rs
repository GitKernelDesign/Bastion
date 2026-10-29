//! Opérations de haut niveau : chaque variante d'[`Op`] est une intention
//! utilisateur, exécutée par le worker sur le transport courant. L'interface
//! ne construit jamais de commande elle-même.

use crate::error::CoreError;
use crate::exec::CommandSpec;
use crate::fail2ban as f2b;
use crate::model::*;
use crate::transport::Transport;
use crate::ufw;

/// Action de service systemd (fail2ban).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceAction {
    Start,
    Stop,
    Restart,
    ReloadOrRestart,
}

impl ServiceAction {
    pub fn systemctl_name(&self) -> &'static str {
        match self {
            ServiceAction::Start => "start",
            ServiceAction::Stop => "stop",
            ServiceAction::Restart => "restart",
            ServiceAction::ReloadOrRestart => "reload-or-restart",
        }
    }
}

/// Une opération demandée par l'interface.
#[derive(Debug, Clone)]
pub enum Op {
    Ping,
    // UFW
    UfwStatus,
    UfwRules,
    UfwEnable,
    UfwDisable,
    UfwReload,
    UfwReset,
    UfwSetDefault {
        dir: Direction,
        policy: Policy,
    },
    UfwSetLogging {
        level: LogLevel,
    },
    UfwAddRule {
        rule: NewRule,
    },
    UfwDeleteRule {
        index: u32,
    },
    UfwAppProfiles,
    UfwReport,
    // Fail2Ban
    F2bStatus,
    /// Détails d'une prison (status + bannedips + paramètres).
    F2bJail {
        jail: String,
    },
    F2bBanIp {
        jail: String,
        ip: String,
    },
    F2bUnbanIp {
        jail: String,
        ip: String,
    },
    F2bBannedAll,
    F2bServiceAction {
        action: ServiceAction,
    },
    /// État systemd du service fail2ban (« active », « inactive »…).
    ServiceState,
    F2bReloadJail {
        jail: Option<String>,
    },
    F2bLogs {
        which: LogSource,
        lines: u32,
    },
}

/// Résultat typé d'une opération.
#[derive(Debug, Clone)]
pub enum OpResult {
    Pong,
    Ack,
    FirewallStatus(Box<FirewallStatus>),
    Rules(Vec<Rule>),
    AppProfiles(Vec<String>),
    Report(String),
    F2bGlobal(Box<F2bGlobal>),
    F2bJail(Box<JailInfo>),
    BannedAll(Vec<(String, Vec<String>)>),
    Text(String),
}

impl Op {
    /// Libellé court pour le journal.
    pub fn describe(&self) -> &'static str {
        match self {
            Op::Ping => "Test de connexion",
            Op::UfwStatus => "UFW : état",
            Op::UfwRules => "UFW : règles",
            Op::UfwEnable => "UFW : activation",
            Op::UfwDisable => "UFW : désactivation",
            Op::UfwReload => "UFW : rechargement",
            Op::UfwReset => "UFW : réinitialisation",
            Op::UfwSetDefault { .. } => "UFW : politique par défaut",
            Op::UfwSetLogging { .. } => "UFW : journalisation",
            Op::UfwAddRule { .. } => "UFW : ajout de règle",
            Op::UfwDeleteRule { .. } => "UFW : suppression de règle",
            Op::UfwAppProfiles => "UFW : profils",
            Op::UfwReport => "UFW : rapport",
            Op::F2bStatus => "Fail2Ban : état",
            Op::F2bJail { .. } => "Fail2Ban : prison",
            Op::F2bBanIp { .. } => "Fail2Ban : bannissement",
            Op::F2bUnbanIp { .. } => "Fail2Ban : débannissement",
            Op::F2bBannedAll => "Fail2Ban : IP bannies",
            Op::F2bServiceAction { .. } => "Fail2Ban : service",
            Op::ServiceState => "Fail2Ban : état du service",
            Op::F2bReloadJail { .. } => "Fail2Ban : rechargement",
            Op::F2bLogs { .. } => "Journaux système",
        }
    }
}

/// Exécute une opération sur un transport.
pub async fn run_op(t: &dyn Transport, op: &Op) -> Result<OpResult, CoreError> {
    match op {
        Op::Ping => {
            let out = crate::transport::exec_lax(t, &CommandSpec::new(crate::exec::Program::Fail2banClient, &["ping"])).await?;
            let _ = out;
            Ok(OpResult::Pong)
        }
        Op::UfwStatus => ufw::status(t).await.map(|s| OpResult::FirewallStatus(Box::new(s))),
        Op::UfwRules => ufw::rules(t).await.map(OpResult::Rules),
        Op::UfwEnable => ufw::enable(t).await.map(|_| OpResult::Ack),
        Op::UfwDisable => ufw::disable(t).await.map(|_| OpResult::Ack),
        Op::UfwReload => ufw::reload(t).await.map(|_| OpResult::Ack),
        Op::UfwReset => ufw::reset(t).await.map(|_| OpResult::Ack),
        Op::UfwSetDefault { dir, policy } => {
            ufw::set_default(t, *dir, *policy).await.map(|_| OpResult::Ack)
        }
        Op::UfwSetLogging { level } => {
            ufw::set_logging(t, *level).await.map(|_| OpResult::Ack)
        }
        Op::UfwAddRule { rule } => ufw::add_rule(t, rule).await.map(|_| OpResult::Ack),
        Op::UfwDeleteRule { index } => {
            ufw::delete_rule(t, *index).await.map(|_| OpResult::Ack)
        }
        Op::UfwAppProfiles => ufw::app_profiles(t).await.map(OpResult::AppProfiles),
        Op::UfwReport => ufw::report(t).await.map(OpResult::Report),
        Op::F2bStatus => f2b::global_status(t).await.map(|g| OpResult::F2bGlobal(Box::new(g))),
        Op::F2bJail { jail } => f2b::jail_status(t, jail).await.map(|j| OpResult::F2bJail(Box::new(j))),
        Op::F2bBanIp { jail, ip } => f2b::ban_ip(t, jail, ip).await.map(|_| OpResult::Ack),
        Op::F2bUnbanIp { jail, ip } => f2b::unban_ip(t, jail, ip).await.map(|_| OpResult::Ack),
        Op::F2bBannedAll => f2b::banned_all(t).await.map(OpResult::BannedAll),
        Op::F2bServiceAction { action } => {
            f2b::service_action(t, action.systemctl_name()).await.map(|_| OpResult::Ack)
        }
        Op::ServiceState => f2b::service_state(t).await.map(OpResult::Text),
        Op::F2bReloadJail { jail } => f2b::reload(t, jail.as_deref()).await.map(|_| OpResult::Ack),
        Op::F2bLogs { which, lines } => {
            f2b::system_logs(t, *which, *lines).await.map(OpResult::Text)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::ExecOutput;
    use async_trait::async_trait;
    use std::sync::Mutex;

    /// Transport factice : réponses scriptées pour les tests.
    struct FakeTransport {
        responses: Mutex<Vec<(CommandSpec, ExecOutput)>>,
    }

    #[async_trait]
    impl Transport for FakeTransport {
        async fn exec(&self, spec: &CommandSpec) -> Result<ExecOutput, CoreError> {
            let mut r = self.responses.lock().unwrap();
            if let Some(pos) = r.iter().position(|(s, _)| s == spec) {
                let (_, out) = r.remove(pos);
                return Ok(out);
            }
            Ok(ExecOutput {
                stdout: String::new(),
                stderr: format!("pas de réponse scriptée pour {}", spec.display()),
                code: 1,
            })
        }
        fn describe(&self) -> String {
            "FakeTransport".into()
        }
    }

    fn t_with(spec: CommandSpec, out: ExecOutput) -> FakeTransport {
        FakeTransport {
            responses: Mutex::new(vec![(spec, out)]),
        }
    }

    fn ok_out(stdout: &str) -> ExecOutput {
        ExecOutput {
            stdout: stdout.into(),
            stderr: String::new(),
            code: 0,
        }
    }

    #[tokio::test]
    async fn op_ufw_status_parse_la_sortie() {
        let t = t_with(
            CommandSpec::new(crate::exec::Program::Ufw, &["status", "verbose"]),
            ok_out("Status: active\nLogging: on (low)\nDefault: deny (incoming), allow (outgoing), deny (routed)\n"),
        );
        let res = run_op(&t, &Op::UfwStatus).await.unwrap();
        match res {
            OpResult::FirewallStatus(s) => {
                assert!(s.active);
                assert_eq!(s.defaults.incoming, Some(Policy::Deny));
                assert!(!s.defaults.routed_disabled);
            }
            _ => panic!("mauvais type de résultat"),
        }
    }

    #[tokio::test]
    async fn op_ufw_add_rule_envoie_les_bons_arguments() {
        // NewRule de test : Proto::Any → forme simple « allow 8080 ».
        let t = t_with(
            CommandSpec::new(crate::exec::Program::Ufw, &["allow", "8080"]),
            ok_out("Rule added\n"),
        );
        let rule = NewRule {
            action: RuleAction::Allow,
            dir: NewRuleDir::In,
            port: Some("8080".into()),
            proto: Proto::Any,
            from_addr: None,
            to_addr: None,
            app_profile: None,
            insert_at: None,
            comment: None,
        };
        run_op(&t, &Op::UfwAddRule { rule }).await.unwrap();
    }

    #[tokio::test]
    async fn op_erreur_remonte_le_stderr() {
        let t = FakeTransport {
            responses: Mutex::new(vec![(
                CommandSpec::new(crate::exec::Program::Ufw, &["enable"]),
                ExecOutput {
                    stdout: String::new(),
                    stderr: "ERROR: problème\n".into(),
                    code: 1,
                },
            )]),
        };
        let err = run_op(&t, &Op::UfwEnable).await.unwrap_err();
        match err {
            CoreError::CommandFailed { stderr, .. } => assert!(stderr.contains("problème")),
            other => panic!("mauvaise erreur : {other:?}"),
        }
    }

    #[tokio::test]
    async fn op_f2b_ban_ip() {
        let t = t_with(
            CommandSpec::new(crate::exec::Program::Fail2banClient, &["set", "sshd", "banip", "1.2.3.4"]),
            ok_out("1\n"),
        );
        run_op(&t, &Op::F2bBanIp { jail: "sshd".into(), ip: "1.2.3.4".into() }).await.unwrap();
    }
}
