//! Internationalisation : FR, EN, ES, DE.
//!
//! Toutes les chaînes visibles de l'application passent par `Tr` ;
//! `localize_error` traduit les erreurs du cœur et complète le stderr
//! système de pistes actionnables.

mod de;
mod en;
mod es;
mod fr;

use bastion_core::model::{Direction, LogLevel, Policy, RuleAction, RuleDir};
use bastion_core::CoreError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lang {
    Fr,
    En,
    Es,
    De,
}

impl Lang {
    /// Détection depuis l'environnement (LANG / LC_ALL / LC_MESSAGES),
    /// repli sur l'anglais.
    pub fn detect() -> Lang {
        for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(v) = std::env::var(key) {
                let v = v.to_ascii_lowercase();
                if v.starts_with("fr") {
                    return Lang::Fr;
                }
                if v.starts_with("es") {
                    return Lang::Es;
                }
                if v.starts_with("de") {
                    return Lang::De;
                }
                if v.starts_with("en") {
                    return Lang::En;
                }
            }
        }
        Lang::En
    }

    /// Code court affiché dans le sélecteur.
    pub fn code(&self) -> &'static str {
        match self {
            Lang::Fr => "FR",
            Lang::En => "EN",
            Lang::Es => "ES",
            Lang::De => "DE",
        }
    }
}

pub fn tr(lang: Lang) -> &'static Tr {
    match lang {
        Lang::Fr => &fr::TR,
        Lang::En => &en::TR,
        Lang::Es => &es::TR,
        Lang::De => &de::TR,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormError {
    InvalidPort,
    InvalidSource,
    InvalidDestination,
    BadInsert,
    ChooseProfile,
    PortRequired,
}

/// Toutes les chaînes de l'interface (une instance statique par langue).
#[derive(Debug)]
pub struct Tr {
    // ── Fenêtre / identité
    pub window_title: &'static str,
    pub app_tagline: &'static str,

    // ── Navigation
    pub nav_connect: &'static str,
    pub nav_firewall: &'static str,
    pub nav_fail2ban: &'static str,
    pub nav_journal: &'static str,
    pub nav_settings: &'static str,

    // ── Barre supérieure / état
    pub status_disconnected: &'static str,
    pub status_connecting: &'static str,
    pub topbar_auto: &'static str,
    pub tooltip_refresh: &'static str,
    pub tooltip_auto: &'static str,
    pub status_ready: &'static str,
    pub ops_pending_t: &'static str, // {n}
    pub shortcut_hint: &'static str,

    // ── Durées relatives
    pub since_now: &'static str,
    pub since_secs_t: &'static str,  // {n}
    pub since_mins_t: &'static str,  // {n}
    pub since_hours_t: &'static str, // {n}

    // ── Étiquettes des énumérations du modèle
    pub policy_allow: &'static str,
    pub policy_deny: &'static str,
    pub policy_reject: &'static str,
    pub action_allow: &'static str,
    pub action_deny: &'static str,
    pub action_reject: &'static str,
    pub action_limit: &'static str,
    pub dir_inbound: &'static str,
    pub dir_outbound: &'static str,
    pub dir_forward: &'static str,
    pub policy_incoming: &'static str,
    pub policy_outgoing: &'static str,
    pub policy_routed: &'static str,
    pub log_off: &'static str,
    pub log_low: &'static str,
    pub log_medium: &'static str,
    pub log_high: &'static str,
    pub log_full: &'static str,
    pub proto_tcp: &'static str,
    pub proto_udp: &'static str,
    pub proto_any: &'static str,

    // ── Écran Connexion
    pub connect_subtitle: &'static str,
    pub session_active: &'static str,
    pub disconnect_btn: &'static str,
    pub target_label: &'static str,
    pub tools_label: &'static str,
    pub session_hint: &'static str,
    pub local_card_title: &'static str,
    pub badge_local: &'static str,
    pub ssh_card_title: &'static str,
    pub connect_btn: &'static str,
    pub local_wait_msg: &'static str,
    pub local_password_hint: &'static str,
    pub profile_placeholder: &'static str,
    pub profile_delete: &'static str,
    pub field_host: &'static str,
    pub field_port: &'static str,
    pub field_user: &'static str,
    pub hint_host: &'static str,
    pub hint_user: &'static str,
    pub auth_password: &'static str,
    pub auth_key: &'static str,
    pub field_key_path: &'static str,
    pub field_passphrase: &'static str,
    pub field_password: &'static str,
    pub use_sudo: &'static str,
    pub use_sudo_hint: &'static str,
    pub field_sudo_password: &'static str,
    pub save_profile_placeholder: &'static str,
    pub save_profile_btn: &'static str,
    pub profile_saved_toast: &'static str,
    pub ssh_fingerprint_hint: &'static str,
    pub privacy_note: &'static str,
    pub probe_unknown_ufw: &'static str,
    pub probe_unknown_f2b: &'static str,

    // ── Écran Pare-feu
    pub fw_subtitle: &'static str,
    pub need_connection_hint: &'static str,
    pub fw_state_active: &'static str,
    pub fw_state_inactive: &'static str,
    pub show_raw_toggle: &'static str,
    pub policies_title: &'static str,
    pub routed_disabled_note: &'static str,
    pub policy_hint: &'static str,
    pub logging_title: &'static str,
    pub logging_hint: &'static str,
    pub rules_title: &'static str,
    pub refresh_rules_tooltip: &'static str,
    pub search_placeholder: &'static str,
    pub col_action: &'static str,
    pub col_from: &'static str,
    pub col_to_port: &'static str,
    pub col_dir: &'static str,
    pub col_port: &'static str,
    pub col_rules: &'static str,
    pub rules_empty: &'static str,
    pub rules_empty_hint: &'static str,
    pub ports_active_title: &'static str,
    pub ports_empty_hint: &'static str,
    pub v6_hint: &'static str,
    pub report_btn: &'static str,
    pub reload_btn: &'static str,
    pub reload_rules_tooltip: &'static str,
    pub reset_btn: &'static str,

    // ── Modales
    pub cancel_btn: &'static str,
    pub close_btn: &'static str,
    pub btn_disable: &'static str,
    pub btn_delete: &'static str,
    pub btn_stop: &'static str,
    pub btn_restart: &'static str,
    pub disable_fw_title: &'static str,
    pub disable_fw_body: &'static str,
    pub restart_f2b_title: &'static str,
    pub restart_f2b_body: &'static str,
    pub stop_f2b_title: &'static str,
    pub stop_f2b_body: &'static str,
    // Service systemd ufw (carte état du pare-feu).
    pub ufw_restart_title: &'static str,
    pub ufw_restart_body: &'static str,
    pub ufw_stop_title: &'static str,
    pub ufw_stop_body: &'static str,
    pub ufw_service_hint: &'static str,
    pub toast_ufw_service_started: &'static str,
    pub toast_ufw_service_stopped: &'static str,
    pub toast_ufw_service_restarted: &'static str,
    // ── ClamAV ──
    pub nav_clamav: &'static str,
    pub clam_subtitle: &'static str,
    pub clam_services_title: &'static str,
    pub clam_db_title: &'static str,
    pub clam_db_engine: &'static str,
    pub clam_db_sigs: &'static str,
    pub clam_db_date: &'static str,
    pub clam_db_unknown: &'static str,
    pub clam_update_btn: &'static str,
    pub clam_update_toast: &'static str,
    pub clam_logs_daemon_btn: &'static str,
    pub clam_logs_freshclam_btn: &'static str,
    pub clam_scan_title: &'static str,
    pub clam_scan_path_label: &'static str,
    pub clam_scan_hint: &'static str,
    pub clam_scan_btn: &'static str,
    pub clam_browse_btn: &'static str,
    pub clam_scan_daemon_note: &'static str,
    pub clam_scan_found_title: &'static str,
    pub clam_scan_clean_toast: &'static str,
    pub clam_scan_infected_fmt: &'static str,
    pub clam_not_installed: &'static str,
    pub clam_service_started: &'static str,
    pub clam_service_stopped: &'static str,
    pub clam_service_restarted: &'static str,
    pub clam_stop_title: &'static str,
    pub clam_stop_body: &'static str,
    pub delete_rule_title: &'static str,
    pub delete_rule_tooltip_t: &'static str, // {desc}
    pub reset_title: &'static str,
    pub reset_body: &'static str,
    pub reset_type_prompt: &'static str,
    pub reset_word: &'static str,
    pub add_rule_title: &'static str,
    pub adv_options: &'static str,
    pub use_profile_cb: &'static str,
    pub field_port_service: &'static str,
    pub hint_port: &'static str,
    pub proto_label: &'static str,
    pub dir_label: &'static str,
    pub action_label: &'static str,
    pub field_source: &'static str,
    pub hint_source: &'static str,
    pub field_dest: &'static str,
    pub hint_dest: &'static str,
    pub field_insert: &'static str,
    pub hint_insert: &'static str,
    pub field_comment: &'static str,
    pub hint_comment: &'static str,
    pub preview_title: &'static str,
    pub btn_add: &'static str,

    // ── Clé de serveur SSH (TOFU)
    pub hk_unknown_title: &'static str,
    pub hk_changed_title: &'static str,
    pub hk_first_time_t: &'static str, // {host}
    pub hk_fp_label: &'static str,
    pub hk_verify_hint: &'static str,
    pub hk_mitm_warning: &'static str,
    pub hk_stored_fp: &'static str,
    pub hk_new_fp: &'static str,
    pub hk_trust_btn: &'static str,

    // ── Écran Fail2Ban
    pub f2b_subtitle: &'static str,
    pub service_label: &'static str,
    pub fw_services_title: &'static str,
    pub fw_status_title: &'static str,
    pub state_unknown: &'static str,
    pub btn_start: &'static str,
    pub jails_title: &'static str,
    pub jails_count_label: &'static str,
    pub banned_count_label: &'static str,
    pub col_jail: &'static str,
    pub col_banned: &'static str,
    pub col_failed: &'static str,
    pub all_ips_btn: &'static str,
    pub all_ips_tooltip: &'static str,
    pub service_log_btn: &'static str,
    pub service_log_tooltip: &'static str,
    pub jails_empty: &'static str,
    pub jails_empty_hint: &'static str,
    pub jail_title_t: &'static str, // {jail}
    pub jail_refresh_tooltip: &'static str,
    pub jail_reload_tooltip: &'static str,
    pub current_failures_label: &'static str,
    pub total_label: &'static str,
    pub maxretry_label: &'static str,
    pub watches_label: &'static str,
    pub reload_service_tooltip: &'static str,
    pub ban_input_hint: &'static str,
    pub btn_ban: &'static str,
    pub btn_unban: &'static str,
    pub banned_list_title: &'static str,
    pub no_banned: &'static str,

    // ── Écran Journal
    pub journal_subtitle: &'static str,
    pub filter_all: &'static str,
    pub filter_errors: &'static str,
    pub filter_fw: &'static str,
    pub filter_f2b: &'static str,
    pub clear_btn: &'static str,
    pub copy_btn: &'static str,
    pub copy_tooltip: &'static str,
    pub journal_copied_toast: &'static str,
    pub journal_empty: &'static str,
    pub journal_empty_hint: &'static str,
    pub entries_count_t: &'static str, // {n}, {cap}

    // ── Écran Paramètres
    pub settings_subtitle: &'static str,
    pub section_appearance: &'static str,
    pub language_label: &'static str,
    pub theme_label: &'static str,
    pub theme_system: &'static str,
    pub theme_dark: &'static str,
    pub theme_light: &'static str,
    pub text_size_label: &'static str,
    pub success_toasts_cb: &'static str,
    pub section_refresh: &'static str,
    pub auto_refresh_cb: &'static str,
    pub interval_label: &'static str,
    pub refresh_hint: &'static str,
    pub section_files: &'static str,
    pub config_path_label: &'static str,
    pub files_hint: &'static str,
    pub section_about: &'static str,
    pub about_line_t: &'static str, // {v}
    pub about_helper: &'static str,

    // ── Toasts d'action
    pub toast_connected: &'static str,
    pub toast_disconnected: &'static str,
    pub toast_conn_lost_t: &'static str,   // {reason}
    pub toast_conn_failed_t: &'static str, // {reason}
    pub toast_fw_enabled: &'static str,
    pub toast_fw_disabled: &'static str,
    pub toast_fw_reloaded: &'static str,
    pub toast_fw_reset: &'static str,
    pub toast_rule_added: &'static str,
    pub toast_rule_deleted: &'static str,
    pub toast_policy_applied_t: &'static str, // {dir}
    pub toast_logging_applied: &'static str,
    pub toast_ip_banned_t: &'static str,   // {ip}, {jail}
    pub toast_ip_unbanned_t: &'static str, // {ip}, {jail}
    pub toast_jail_reloaded: &'static str,
    pub toast_service_started: &'static str,
    pub toast_service_stopped: &'static str,
    pub toast_service_restarted: &'static str,
    pub toast_service_reloaded: &'static str,
    pub undo_btn: &'static str,
    pub undo_tooltip: &'static str,

    // ── Erreurs
    pub err_not_connected: &'static str,
    pub err_program_not_found_t: &'static str, // {program}
    pub err_parse_t: &'static str,             // {program}, {detail}
    pub err_disconnected_t: &'static str,      // {detail}
    pub err_not_whitelisted_t: &'static str,   // {detail}
    pub err_ssh_auth_t: &'static str,          // {user}
    pub err_hostkey_rejected: &'static str,
    pub hint_root_required: &'static str,
    pub hint_sudo_password: &'static str,
    pub hint_tool_missing: &'static str,

    // ── Erreurs du formulaire de règle
    pub ferr_invalid_port: &'static str,
    pub ferr_invalid_source: &'static str,
    pub ferr_invalid_destination: &'static str,
    pub ferr_bad_insert: &'static str,
    pub ferr_choose_profile: &'static str,
    pub ferr_port_required: &'static str,
}

fn rep(t: &str, pairs: &[(&str, &str)]) -> String {
    let mut s = t.to_string();
    for (k, v) in pairs {
        s = s.replace(k, v);
    }
    s
}

impl Tr {
    pub fn since(&self, secs: u64) -> String {
        match secs {
            0 => self.since_now.to_string(),
            s if s < 60 => rep(self.since_secs_t, &[("{n}", &s.to_string())]),
            m if m < 3600 => rep(self.since_mins_t, &[("{n}", &(m / 60).to_string())]),
            h => rep(self.since_hours_t, &[("{n}", &(h / 3600).to_string())]),
        }
    }

    pub fn ops_pending(&self, n: usize) -> String {
        rep(self.ops_pending_t, &[("{n}", &n.to_string())])
    }

    /// Toast de fin d'analyse avec fichiers infectés.
    pub fn clam_scan_infected_toast(&self, n: usize) -> String {
        rep(self.clam_scan_infected_fmt, &[("{n}", &n.to_string())])
    }

    pub fn rules_with_count(&self, n: usize) -> String {
        format!("{} ({})", self.rules_title, n)
    }

    pub fn jails_with_count(&self, n: usize) -> String {
        format!("{} ({})", self.jails_title, n)
    }

    pub fn entries_count(&self, n: usize, cap: usize) -> String {
        rep(
            self.entries_count_t,
            &[("{n}", &n.to_string()), ("{cap}", &cap.to_string())],
        )
    }

    pub fn policy_applied(&self, dir: Direction) -> String {
        let d = match dir {
            Direction::Incoming => self.policy_incoming,
            Direction::Outgoing => self.policy_outgoing,
            Direction::Routed => self.policy_routed,
        };
        rep(self.toast_policy_applied_t, &[("{dir}", d)])
    }

    pub fn ip_banned_toast(&self, ip: &str, jail: &str) -> String {
        rep(self.toast_ip_banned_t, &[("{ip}", ip), ("{jail}", jail)])
    }

    pub fn ip_unbanned_toast(&self, ip: &str, jail: &str) -> String {
        rep(self.toast_ip_unbanned_t, &[("{ip}", ip), ("{jail}", jail)])
    }

    pub fn jail_title(&self, jail: &str) -> String {
        rep(self.jail_title_t, &[("{jail}", jail)])
    }

    pub fn delete_rule_tooltip(&self, desc: &str) -> String {
        rep(self.delete_rule_tooltip_t, &[("{desc}", desc)])
    }

    pub fn first_time_key(&self, host: &str) -> String {
        rep(self.hk_first_time_t, &[("{host}", host)])
    }

    pub fn about_line(&self, version: &str) -> String {
        rep(self.about_line_t, &[("{v}", version)])
    }

    pub fn conn_lost(&self, reason: &str) -> String {
        rep(self.toast_conn_lost_t, &[("{reason}", reason)])
    }

    pub fn conn_failed(&self, reason: &str) -> String {
        rep(self.toast_conn_failed_t, &[("{reason}", reason)])
    }

    /// Traduit une erreur du cœur pour l'affichage, avec pistes actionnables.
    pub fn localize_error(&self, e: &CoreError) -> String {
        match e {
            CoreError::NotConnected => self.err_not_connected.to_string(),
            CoreError::ProgramNotFound { program } => {
                rep(self.err_program_not_found_t, &[("{program}", program)])
            }
            CoreError::ParseFailed { program, detail } => rep(
                self.err_parse_t,
                &[("{program}", program), ("{detail}", detail)],
            ),
            CoreError::Disconnected(detail) => {
                rep(self.err_disconnected_t, &[("{detail}", detail)])
            }
            CoreError::NotWhitelisted(detail) => {
                rep(self.err_not_whitelisted_t, &[("{detail}", detail)])
            }
            CoreError::SshAuthFailed { user } => rep(self.err_ssh_auth_t, &[("{user}", user)]),
            CoreError::HostKeyRejected => self.err_hostkey_rejected.to_string(),
            CoreError::CommandFailed { stderr, .. } => {
                // Le message système, complété d'une piste si la cause est connue.
                let mut msg = stderr.clone();
                if msg.is_empty() {
                    msg = e.to_string();
                }
                if msg.contains("You need to be root") {
                    msg.push_str("\n→ ");
                    msg.push_str(self.hint_root_required);
                } else if msg.contains("a password is required")
                    || msg.contains("sudo: no tty")
                    || msg.contains("no password was provided")
                {
                    msg.push_str("\n→ ");
                    msg.push_str(self.hint_sudo_password);
                } else if msg.contains("could not be found") || msg.contains("command not found") {
                    msg.push_str("\n→ ");
                    msg.push_str(self.hint_tool_missing);
                }
                msg
            }
            CoreError::Other(detail) => detail.clone(),
        }
    }

    pub fn form_error(&self, e: FormError) -> &'static str {
        match e {
            FormError::InvalidPort => self.ferr_invalid_port,
            FormError::InvalidSource => self.ferr_invalid_source,
            FormError::InvalidDestination => self.ferr_invalid_destination,
            FormError::BadInsert => self.ferr_bad_insert,
            FormError::ChooseProfile => self.ferr_choose_profile,
            FormError::PortRequired => self.ferr_port_required,
        }
    }

    // ── Libellés des énumérations du modèle ──
    pub fn policy(&self, p: Policy) -> &'static str {
        match p {
            Policy::Allow => self.policy_allow,
            Policy::Deny => self.policy_deny,
            Policy::Reject => self.policy_reject,
        }
    }

    pub fn action(&self, a: RuleAction) -> &'static str {
        match a {
            RuleAction::Allow => self.action_allow,
            RuleAction::Deny => self.action_deny,
            RuleAction::Reject => self.action_reject,
            RuleAction::Limit => self.action_limit,
        }
    }

    pub fn rule_dir(&self, d: RuleDir) -> &'static str {
        match d {
            RuleDir::In => self.dir_inbound,
            RuleDir::Out => self.dir_outbound,
            RuleDir::Fwd => self.dir_forward,
        }
    }

    pub fn log_level(&self, l: LogLevel) -> &'static str {
        match l {
            LogLevel::Off => self.log_off,
            LogLevel::Low => self.log_low,
            LogLevel::Medium => self.log_medium,
            LogLevel::High => self.log_high,
            LogLevel::Full => self.log_full,
        }
    }
}