//! État global de l'interface et boucle de rendu.

use crate::config::AppConfig;
use crate::i18n::{tr, FormError, Lang, Tr};
use crate::toast::{ToastAction, Toasts};
use crate::worker::{Job, LogEntry, WorkerHandle, Event};
use bastion_core::model::*;
use bastion_core::op::{Op, OpResult, ServiceAction};
use bastion_core::ssh::SshParams;
use std::collections::{HashMap, VecDeque};
use std::time::Instant;

pub const JOURNAL_CAP: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Connect,
    Firewall,
    Fail2Ban,
    Journal,
    Settings,
}

impl Screen {
    pub fn from_index(i: usize) -> Screen {
        match i {
            1 => Screen::Firewall,
            2 => Screen::Fail2Ban,
            3 => Screen::Journal,
            4 => Screen::Settings,
            _ => Screen::Connect,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnPhase {
    Disconnected,
    Connecting,
    Connected,
}

/// Ce que l'interface attend du résultat d'une opération en vol.
#[derive(Debug, Clone)]
pub enum Pending {
    RefreshStatus,
    RefreshRules,
    RefreshProfiles,
    RefreshF2b,
    RefreshService,
    RefreshJail,
    Enable,
    Disable,
    Reload,
    Reset,
    Policy(Direction),
    Logging,
    AddRule,
    DeleteRule,
    Report,
    Ban { jail: String, ip: String },
    Unban { jail: String, ip: String },
    BannedAll,
    Service(ServiceAction),
    UfwService(ServiceAction),
    RefreshUfwService,
    ReloadJail,
    Logs,
}

/// Modales.
pub enum Modal {
    Confirm(ConfirmData),
    ResetFw { typed: String },
    AddRule,
    Report(String),
    Logs { title: String, text: String },
    HostKey(HostKeyCtx),
}

pub struct ConfirmData {
    pub title: String,
    pub body: String,
    pub confirm_label: String,
    pub kind: ConfirmKind,
}

#[derive(Debug, Clone)]
pub enum ConfirmKind {
    DisableFw,
    DeleteRule { index: u32 },
    StopF2b,
    RestartF2b,
    StopUfw,
    RestartUfw,
}

pub struct HostKeyCtx {
    pub host: String,
    pub fingerprint: String,
    pub old: Option<String>,
    pub params: SshParams,
}

/// Données de l'écran Pare-feu.
pub struct FirewallUi {
    pub status: Option<FirewallStatus>,
    pub status_at: Option<Instant>,
    pub rules: Vec<Rule>,
    pub rules_at: Option<Instant>,
    pub profiles: Vec<String>,
    pub search: String,
    pub show_raw: bool,
    /// État systemd du service ufw (« active », « inactive »…).
    pub ufw_service: Option<String>,
    pub ufw_service_at: Option<Instant>,
}

/// Données de l'écran Fail2Ban.
pub struct F2bUi {
    pub global: Option<F2bGlobal>,
    pub global_at: Option<Instant>,
    pub service: Option<String>,
    pub service_at: Option<Instant>,
    pub jail_infos: HashMap<String, (JailInfo, Instant)>,
    pub selected_jail: Option<String>,
    pub banned_all: Option<Vec<(String, Vec<String>)>>,
    pub ban_input: String,
}

/// Formulaire de connexion.
pub struct ConnectForm {
    pub mode_ssh: bool,
    pub profile_name: Option<String>,
    pub host: String,
    pub port: String,
    pub user: String,
    pub auth_key: bool,
    pub password: String,
    pub key_path: String,
    pub passphrase: String,
    pub use_sudo: bool,
    pub sudo_password: String,
    pub save_name: String,
}

impl Default for ConnectForm {
    fn default() -> Self {
        ConnectForm {
            mode_ssh: false,
            profile_name: None,
            host: String::new(),
            port: "22".into(),
            user: String::new(),
            auth_key: false,
            password: String::new(),
            key_path: String::new(),
            passphrase: String::new(),
            use_sudo: false,
            sudo_password: String::new(),
            save_name: String::new(),
        }
    }
}

/// Formulaire d'ajout de règle.
pub struct AddRuleForm {
    pub action: RuleAction,
    pub dir: NewRuleDir,
    pub port: String,
    pub proto: Proto,
    pub advanced: bool,
    pub from_addr: String,
    pub to_addr: String,
    pub insert_at: String,
    pub comment: String,
    pub use_profile: bool,
    pub profile: Option<String>,
}

impl Default for AddRuleForm {
    fn default() -> Self {
        AddRuleForm {
            action: RuleAction::Allow,
            dir: NewRuleDir::In,
            port: String::new(),
            proto: Proto::Tcp,
            advanced: false,
            from_addr: String::new(),
            to_addr: String::new(),
            insert_at: String::new(),
            comment: String::new(),
            use_profile: false,
            profile: None,
        }
    }
}

impl AddRuleForm {
    /// Construit la règle validée, ou l'erreur localisable.
    pub fn to_new_rule(&self) -> Result<NewRule, FormError> {
        if self.use_profile {
            let Some(profile) = &self.profile else {
                return Err(FormError::ChooseProfile);
            };
            return Ok(NewRule {
                action: self.action,
                dir: self.dir,
                port: None,
                proto: Proto::Any,
                from_addr: None,
                to_addr: None,
                app_profile: Some(profile.clone()),
                insert_at: self.parse_insert_at()?,
                comment: self.comment_text(),
            });
        }
        let port = self.port.trim();
        if port.is_empty() {
            return Err(FormError::PortRequired);
        }
        if !bastion_core::ufw::is_valid_port_spec(port) {
            return Err(FormError::InvalidPort);
        }
        if let Some(a) = self.from_addr.trim().strip_suffix("") {
            if !a.is_empty() && !bastion_core::ufw::is_valid_addr(a) {
                return Err(FormError::InvalidSource);
            }
        }
        if let Some(a) = self.to_addr.trim().strip_suffix("") {
            if !a.is_empty() && !bastion_core::ufw::is_valid_addr(a) {
                return Err(FormError::InvalidDestination);
            }
        }
        Ok(NewRule {
            action: self.action,
            dir: self.dir,
            port: Some(port.to_string()),
            proto: self.proto,
            from_addr: non_empty(&self.from_addr),
            to_addr: non_empty(&self.to_addr),
            app_profile: None,
            insert_at: self.parse_insert_at()?,
            comment: self.comment_text(),
        })
    }

    fn parse_insert_at(&self) -> Result<Option<u32>, FormError> {
        let s = self.insert_at.trim();
        if s.is_empty() {
            return Ok(None);
        }
        s.parse::<u32>()
            .ok()
            .filter(|n| *n >= 1)
            .map(Some)
            .ok_or(FormError::BadInsert)
    }

    fn comment_text(&self) -> Option<String> {
        let c = self.comment.trim();
        if c.is_empty() {
            None
        } else {
            Some(c.to_string())
        }
    }

    /// Aperçu exact de la commande (police mono), ou de l'erreur.
    pub fn preview(&self, t: &Tr) -> String {
        match self.to_new_rule() {
            Ok(rule) => match bastion_core::ufw::build_rule_args(&rule) {
                Ok(args) => {
                    let mut s = String::from("ufw");
                    for a in args {
                        s.push(' ');
                        s.push_str(&bastion_core::exec::quote_shell(&a));
                    }
                    s
                }
                Err(e) => t.localize_error(&e),
            },
            Err(e) => format!("— {}", t.form_error(e)),
        }
    }
}

fn non_empty(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

pub struct GuiApp {
    pub worker: WorkerHandle,
    pub cfg: AppConfig,
    pub lang: Lang,
    pub tr: &'static Tr,
    pub screen: Screen,
    pub phase: ConnPhase,
    pub conn_desc: String,
    pub expect_disconnect: bool,
    pub last_refresh_sent: Option<Instant>,
    pub pending: HashMap<u64, Pending>,
    pub next_id: u64,
    pub journal: VecDeque<LogEntry>,
    pub filter: crate::app::LogFilter,
    pub toasts: Toasts,
    pub modal: Option<Modal>,
    pub firewall: FirewallUi,
    pub f2b: F2bUi,
    pub form: ConnectForm,
    pub add_form: AddRuleForm,
    pub probe: (Option<String>, Option<String>),
    pub status_msg: String,
    pub last_error_toast: Option<(String, Instant)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFilter {
    All,
    Errors,
    Fw,
    F2b,
}

impl GuiApp {
    pub fn new(cc: &eframe::CreationContext) -> GuiApp {
        crate::fonts::install(&cc.egui_ctx);
        let cfg = AppConfig::load();
        let lang = cfg.language;
        let t = tr(lang);
        crate::theme::apply(&cc.egui_ctx, cfg.theme, cfg.font_scale);
        let worker = WorkerHandle::spawn();
        worker.send(Job::ProbeLocalTools);

        GuiApp {
            worker,
            cfg,
            lang,
            tr: t,
            screen: Screen::Connect,
            phase: ConnPhase::Disconnected,
            conn_desc: String::new(),
            expect_disconnect: false,
            last_refresh_sent: None,
            pending: HashMap::new(),
            next_id: 1,
            journal: VecDeque::with_capacity(JOURNAL_CAP),
            filter: LogFilter::All,
            toasts: Toasts::default(),
            modal: None,
            firewall: FirewallUi {
                status: None,
                status_at: None,
                rules: Vec::new(),
                rules_at: None,
                profiles: Vec::new(),
                search: String::new(),
                show_raw: false,
                ufw_service: None,
                ufw_service_at: None,
            },
            f2b: F2bUi {
                global: None,
                global_at: None,
                service: None,
                service_at: None,
                jail_infos: HashMap::new(),
                selected_jail: None,
                banned_all: None,
                ban_input: String::new(),
            },
            form: ConnectForm::default(),
            add_form: AddRuleForm::default(),
            probe: (None, None),
            status_msg: String::new(),
            last_error_toast: None,
        }
    }

    /// Changement de langue : traductions + titre de fenêtre.
    pub fn set_language(&mut self, ctx: &egui::Context, lang: Lang) {
        self.lang = lang;
        self.cfg.language = lang;
        self.cfg.save();
        self.tr = tr(lang);
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.tr.window_title.to_string()));
    }

    // ── Envoi d'opérations ────────────────────────────────────────────────
    pub fn next_op_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn send_op(&mut self, pending: Pending, op: Op) {
        let id = self.next_op_id();
        self.pending.insert(id, pending);
        self.worker.send(Job::Op { id, op });
    }

    pub fn refresh_all(&mut self) {
        self.last_refresh_sent = Some(Instant::now());
        self.send_op(Pending::RefreshStatus, Op::UfwStatus);
        self.send_op(Pending::RefreshRules, Op::UfwRules);
        self.send_op(Pending::RefreshF2b, Op::F2bStatus);
        self.send_op(Pending::RefreshService, Op::ServiceState);
        self.send_op(Pending::RefreshUfwService, Op::UfwServiceState);
        if let Some(jail) = self.f2b.selected_jail.clone() {
            self.send_op(Pending::RefreshJail, Op::F2bJail { jail });
        }
    }

    pub fn is_refreshing(&self) -> bool {
        self.pending
            .values()
            .any(|p| matches!(p, Pending::RefreshStatus | Pending::RefreshRules | Pending::RefreshF2b | Pending::RefreshService | Pending::RefreshUfwService | Pending::RefreshJail))
    }

    // ── Journal / messages ────────────────────────────────────────────────
    pub fn push_journal(&mut self, entry: LogEntry) {
        while self.journal.len() >= JOURNAL_CAP {
            self.journal.pop_front();
        }
        self.journal.push_back(entry);
    }

    fn toast_error_throttled(&mut self, msg: String) {
        let repeat_ok = self
            .last_error_toast
            .as_ref()
            .map(|(_, t)| t.elapsed().as_secs() > 30)
            .unwrap_or(true);
        let different = self
            .last_error_toast
            .as_ref()
            .map(|(m, _)| m != &msg)
            .unwrap_or(true);
        if repeat_ok || different {
            self.toasts.error(msg.clone());
            self.last_error_toast = Some((msg.clone(), Instant::now()));
        }
        self.status_msg = msg;
    }

    // ── Événements du worker ──────────────────────────────────────────────
    pub fn apply_event(&mut self, ev: Event) {
        match ev {
            Event::Connected { desc } => {
                self.phase = ConnPhase::Connected;
                self.conn_desc = desc;
                self.form.password.clear();
                self.form.passphrase.clear();
                self.form.sudo_password.clear();
                self.modal = None;
                self.screen = Screen::Firewall;
                self.toasts.success(self.tr.toast_connected.to_string());
                self.refresh_all();
            }
            Event::ConnectFailed { reason } => {
                self.phase = ConnPhase::Disconnected;
                if let Some(Modal::HostKey(ctx)) = &self.modal {
                    if ctx.fingerprint.is_empty() {
                        self.modal = None;
                    }
                }
                self.toasts.error(self.tr.conn_failed(&reason));
            }
            Event::HostKeyUnknown { fingerprint, .. } => {
                self.phase = ConnPhase::Disconnected;
                if let Some(Modal::HostKey(ctx)) = &mut self.modal {
                    ctx.fingerprint = fingerprint;
                }
            }
            Event::HostKeyChanged { new, old, .. } => {
                self.phase = ConnPhase::Disconnected;
                if let Some(Modal::HostKey(ctx)) = &mut self.modal {
                    ctx.fingerprint = new;
                    ctx.old = Some(old);
                }
            }
            Event::Disconnected { reason } => {
                let initiated = self.expect_disconnect;
                self.expect_disconnect = false;
                self.phase = ConnPhase::Disconnected;
                self.conn_desc.clear();
                if initiated {
                    self.toasts.info(self.tr.toast_disconnected.to_string());
                } else {
                    self.toasts.error(self.tr.conn_lost(&reason));
                }
            }
            Event::OpDone { id, result } => {
                if let Some(pending) = self.pending.remove(&id) {
                    self.apply_op_result(pending, result);
                }
            }
            Event::Log(entry) => self.push_journal(entry),
            Event::ProbeLocalTools { ufw, fail2ban } => self.probe = (ufw, fail2ban),
        }
    }

    fn apply_op_result(&mut self, pending: Pending, result: Result<OpResult, bastion_core::CoreError>) {
        match result {
            Ok(res) => self.apply_op_ok(pending, res),
            Err(e) => self.apply_op_err(pending, e),
        }
    }

    fn apply_op_ok(&mut self, pending: Pending, res: OpResult) {
        match pending {
            Pending::RefreshStatus => {
                if let OpResult::FirewallStatus(s) = res {
                    self.firewall.status = Some(*s);
                    self.firewall.status_at = Some(Instant::now());
                }
            }
            Pending::RefreshRules => {
                if let OpResult::Rules(r) = res {
                    self.firewall.rules = r;
                    self.firewall.rules_at = Some(Instant::now());
                }
            }
            Pending::RefreshProfiles => {
                if let OpResult::AppProfiles(p) = res {
                    self.firewall.profiles = p;
                }
            }
            Pending::RefreshF2b => {
                if let OpResult::F2bGlobal(g) = res {
                    if let Some(sel) = &self.f2b.selected_jail {
                        if !g.jails.contains(sel) {
                            self.f2b.selected_jail = None;
                        }
                    }
                    self.f2b.global = Some(*g);
                    self.f2b.global_at = Some(Instant::now());
                }
            }
            Pending::RefreshService => {
                if let OpResult::Text(t) = res {
                    self.f2b.service = Some(t);
                    self.f2b.service_at = Some(Instant::now());
                }
            }
            Pending::RefreshUfwService => {
                if let OpResult::Text(s) = res {
                    self.firewall.ufw_service = Some(s);
                    self.firewall.ufw_service_at = Some(Instant::now());
                }
            }
            Pending::RefreshJail => {
                if let OpResult::F2bJail(j) = res {
                    self.f2b.jail_infos.insert(j.name.clone(), (*j, Instant::now()));
                }
            }
            Pending::Enable => {
                self.success(self.tr.toast_fw_enabled);
                self.send_op(Pending::RefreshStatus, Op::UfwStatus);
                self.send_op(Pending::RefreshRules, Op::UfwRules);
            }
            Pending::Disable => {
                self.success(self.tr.toast_fw_disabled);
                self.send_op(Pending::RefreshStatus, Op::UfwStatus);
            }
            Pending::Reload => {
                self.success(self.tr.toast_fw_reloaded);
                self.send_op(Pending::RefreshStatus, Op::UfwStatus);
                self.send_op(Pending::RefreshRules, Op::UfwRules);
            }
            Pending::Reset => {
                self.success(self.tr.toast_fw_reset);
                self.refresh_all();
            }
            Pending::Policy(dir) => {
                let msg = self.tr.policy_applied(dir);
                self.success(&msg);
                self.send_op(Pending::RefreshStatus, Op::UfwStatus);
                self.send_op(Pending::RefreshRules, Op::UfwRules);
            }
            Pending::Logging => {
                self.success(self.tr.toast_logging_applied);
                self.send_op(Pending::RefreshStatus, Op::UfwStatus);
            }
            Pending::AddRule => {
                self.success(self.tr.toast_rule_added);
                self.send_op(Pending::RefreshStatus, Op::UfwStatus);
                self.send_op(Pending::RefreshRules, Op::UfwRules);
            }
            Pending::DeleteRule => {
                self.success(self.tr.toast_rule_deleted);
                self.send_op(Pending::RefreshStatus, Op::UfwStatus);
                self.send_op(Pending::RefreshRules, Op::UfwRules);
            }
            Pending::Report => {
                if let OpResult::Report(text) = res {
                    self.modal = Some(Modal::Report(text));
                }
            }
            Pending::Ban { jail, ip } => {
                self.toasts.success_undo(
                    self.tr.ip_banned_toast(&ip, &jail),
                    ToastAction::Reunban { jail: jail.clone(), ip },
                );
                self.send_op(Pending::RefreshJail, Op::F2bJail { jail });
                self.send_op(Pending::RefreshF2b, Op::F2bStatus);
            }
            Pending::Unban { jail, ip } => {
                self.toasts.success_undo(
                    self.tr.ip_unbanned_toast(&ip, &jail),
                    ToastAction::Reban { jail: jail.clone(), ip },
                );
                self.send_op(Pending::RefreshJail, Op::F2bJail { jail });
                self.send_op(Pending::RefreshF2b, Op::F2bStatus);
            }
            Pending::BannedAll => {
                if let OpResult::BannedAll(b) = res {
                    let total: usize = b.iter().map(|(_, ips)| ips.len()).sum();
                    let mut text = format!("{}\n", self.tr.banned_list_title);
                    for (jail, ips) in &b {
                        if ips.is_empty() {
                            continue;
                        }
                        text.push_str(&format!("\n{} ({}):\n", jail, ips.len()));
                        for ip in ips {
                            text.push_str(&format!("  {ip}\n"));
                        }
                    }
                    if total == 0 {
                        text.push_str(&format!("\n{}", self.tr.no_banned));
                    }
                    self.f2b.banned_all = Some(b);
                    self.modal = Some(Modal::Logs {
                        title: self.tr.all_ips_btn.to_string(),
                        text,
                    });
                }
            }
            Pending::Service(action) => {
                let msg = match action {
                    ServiceAction::Start => self.tr.toast_service_started,
                    ServiceAction::Stop => self.tr.toast_service_stopped,
                    ServiceAction::Restart => self.tr.toast_service_restarted,
                    ServiceAction::ReloadOrRestart => self.tr.toast_service_reloaded,
                };
                self.success(msg);
                self.send_op(Pending::RefreshService, Op::ServiceState);
                self.send_op(Pending::RefreshF2b, Op::F2bStatus);
                if let Some(jail) = self.f2b.selected_jail.clone() {
                    self.send_op(Pending::RefreshJail, Op::F2bJail { jail });
                }
            }
            Pending::UfwService(action) => {
                let msg = match action {
                    ServiceAction::Start => self.tr.toast_ufw_service_started,
                    ServiceAction::Stop => self.tr.toast_ufw_service_stopped,
                    ServiceAction::Restart => self.tr.toast_ufw_service_restarted,
                    ServiceAction::ReloadOrRestart => self.tr.toast_ufw_service_restarted,
                };
                self.success(msg);
                self.send_op(Pending::RefreshUfwService, Op::UfwServiceState);
                self.send_op(Pending::RefreshStatus, Op::UfwStatus);
                self.send_op(Pending::RefreshRules, Op::UfwRules);
            }
            Pending::ReloadJail => {
                self.success(self.tr.toast_jail_reloaded);
                self.send_op(Pending::RefreshService, Op::ServiceState);
                self.send_op(Pending::RefreshF2b, Op::F2bStatus);
                if let Some(jail) = self.f2b.selected_jail.clone() {
                    self.send_op(Pending::RefreshJail, Op::F2bJail { jail });
                }
            }
            Pending::Logs => {
                if let OpResult::Text(text) = res {
                    self.modal = Some(Modal::Logs {
                        title: self.tr.service_log_btn.to_string(),
                        text,
                    });
                }
            }
        }
    }

    fn success(&mut self, msg: &str) {
        if self.cfg.success_toasts {
            self.toasts.success(msg.to_string());
        }
        self.status_msg = msg.to_string();
    }

    fn apply_op_err(&mut self, pending: Pending, e: bastion_core::CoreError) {
        let msg = self.tr.localize_error(&e);
        let is_refresh = matches!(
            pending,
            Pending::RefreshStatus
                | Pending::RefreshRules
                | Pending::RefreshProfiles
                | Pending::RefreshF2b
                | Pending::RefreshService
                | Pending::RefreshUfwService
                | Pending::RefreshJail
        );
        if is_refresh {
            self.toast_error_throttled(msg);
        } else {
            self.toasts.error(msg.clone());
            self.status_msg = msg;
        }
    }
}

impl eframe::App for GuiApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        for ev in self.worker.poll() {
            self.apply_event(ev);
        }

        // Actualisation automatique.
        if self.phase == ConnPhase::Connected && self.cfg.auto_refresh {
            let due = self
                .last_refresh_sent
                .map(|t| t.elapsed().as_secs() >= self.cfg.refresh_secs)
                .unwrap_or(true);
            if due && !self.is_refreshing() {
                self.refresh_all();
            }
        }

        self.handle_shortcuts(&ctx);

        let p = crate::theme::Palette::new(self.cfg.theme);
        crate::ui::render(self, ui, &p);

        if let Some(modal) = self.modal.take() {
            crate::modal::render(self, &ctx, &p, modal);
        }
        let actions = self.toasts.show(&ctx, &p, self.tr);
        for action in actions {
            match action {
                ToastAction::Reban { jail, ip } => {
                    self.send_op(
                        Pending::Ban { jail: jail.clone(), ip: ip.clone() },
                        Op::F2bBanIp { jail, ip },
                    );
                }
                ToastAction::Reunban { jail, ip } => {
                    self.send_op(
                        Pending::Unban { jail: jail.clone(), ip: ip.clone() },
                        Op::F2bUnbanIp { jail, ip },
                    );
                }
            }
        }

        if self.phase == ConnPhase::Connected && self.cfg.auto_refresh {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
    }

}

impl GuiApp {
    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        // Échap ferme la modale, même pendant la saisie.
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && self.modal.is_some() {
            self.modal = None;
            return;
        }
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let (ctrl, refresh, new_rule, num) = ctx.input(|i| {
            let num = [
                egui::Key::Num1,
                egui::Key::Num2,
                egui::Key::Num3,
                egui::Key::Num4,
                egui::Key::Num5,
            ]
            .iter()
            .position(|k| i.key_pressed(*k));
            (
                i.modifiers.command || i.modifiers.ctrl,
                i.key_pressed(egui::Key::R),
                i.key_pressed(egui::Key::N),
                num,
            )
        });
        if !ctrl {
            return;
        }
        if refresh && self.phase == ConnPhase::Connected {
            self.refresh_all();
        }
        if new_rule && self.phase == ConnPhase::Connected && self.screen == Screen::Firewall {
            self.add_form = AddRuleForm::default();
            if self.firewall.profiles.is_empty() {
                self.send_op(Pending::RefreshProfiles, Op::UfwAppProfiles);
            }
            self.modal = Some(Modal::AddRule);
        }
        if let Some(n) = num {
            self.screen = Screen::from_index(n);
        }
    }
}