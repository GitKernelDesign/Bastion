//! Écran Pare-feu.
//!
//! Hiérarchie : un « héro » (état + interrupteur + politiques + actions
//! principales), trois chiffres clés, puis les réglages (politiques,
//! journalisation) et enfin les règles avec leur composeur repliable.
//! Les détails techniques (sortie brute, service systemd) sont repliés.

use crate::app::{ConfirmData, ConfirmKind, GuiApp, Modal, Pending};
use crate::components;
use crate::icons;
use crate::theme::{space, text, Palette};
use bastion_core::model::*;
use bastion_core::op::{Op, ServiceAction};
use egui::{Align, CornerRadius, Frame, Layout, Margin, RichText, Stroke};

pub fn render(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;

    if app.phase != crate::app::ConnPhase::Connected {
        components::card_body(ui, p, |ui| {
            if components::need_connection(ui, p, t).clicked() {
                app.screen = crate::app::Screen::Connect;
            }
        });
        return;
    }

    // 1. Bandeau d'alerte quand le pare-feu est désactivé.
    let active = app.firewall.status.as_ref().map(|s| s.active).unwrap_or(false);
    if app.firewall.status.is_some() && !active {
        components::banner(
            ui,
            p,
            p.danger,
            icons::GPP_BAD,
            t.fw_off_title,
            t.fw_off_body,
            |ui| {
                if components::primary_button(
                    ui,
                    p,
                    &format!("{}  {}", icons::POWER, t.fw_enable_now),
                )
                .clicked()
                {
                    app.send_op(Pending::Enable, Op::UfwEnable);
                }
            },
        );
        ui.add_space(space::M);
    }

    // 2. Carte d'état principale.
    status_card(app, ui, p);
    ui.add_space(space::M);

    // 3. Chiffres clés.
    stats_row(app, ui, p);
    ui.add_space(space::M);

    // 4. Politiques par défaut (journalisation comprise).
    policy_card(app, ui, p);
    ui.add_space(space::M);

    // 5. Règles (composeur + table).
    rules_card(app, ui, p);
    ui.add_space(space::M);

    // 6. Actions rares et techniques.
    technical_card(app, ui, p);
}

// ─── Carte d'état ────────────────────────────────────────────────────────────
fn status_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    let active = app.firewall.status.as_ref().map(|s| s.active).unwrap_or(false);
    let known = app.firewall.status.is_some();
    let version = app.probe.0.clone().unwrap_or_default();
    let svc = app.firewall.ufw_service.clone();
    let status_at = app.firewall.status_at;
    let busy = app.pending.values().any(|x| {
        matches!(
            x,
            Pending::Enable | Pending::Disable | Pending::RefreshStatus | Pending::UfwService(_)
        )
    });
    let mut refresh = false;

    components::card(
        ui,
        p,
        t.fw_status_title,
        |ui| {
            if !version.is_empty() {
                ui.label(
                    RichText::new(&version)
                        .size(text::MICRO)
                        .monospace()
                        .color(p.text_secondary),
                );
            }
            if let Some(at) = status_at {
                components::since_label(ui, p, t, at);
            }
            if components::icon_button(ui, p, icons::REFRESH, t.tooltip_refresh).clicked() {
                refresh = true;
            }
        },
        |ui| {
            ui.horizontal(|ui| {
                let (color, title, sub) = if !known {
                    (p.text_secondary, t.state_unknown.to_string(), String::new())
                } else if active {
                    (
                        p.success,
                        t.fw_state_active.to_string(),
                        match &svc {
                            Some(s) if s == "active" => format!("{} · {s}", t.fw_service_title),
                            Some(s) => format!("{} · {s}", t.fw_service_title),
                            None => t.fw_service_title.to_string(),
                        },
                    )
                } else {
                    (
                        p.danger,
                        t.fw_state_inactive.to_string(),
                        t.fw_off_title.to_string(),
                    )
                };
                ui.vertical(|ui| {
                    components::status_line(ui, p, color, &title, None);
                    if !sub.is_empty() {
                        ui.label(RichText::new(sub).size(text::SMALL).color(p.text_secondary));
                    }
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let on = components::switch_enabled(
                        ui,
                        p,
                        active,
                        Some(p.success),
                        known && !busy,
                    );
                    if on != active && known && !busy {
                        if on {
                            app.send_op(Pending::Enable, Op::UfwEnable);
                        } else {
                            app.modal = Some(Modal::Confirm(ConfirmData {
                                title: t.disable_fw_title.to_string(),
                                body: t.disable_fw_body.to_string(),
                                confirm_label: t.btn_disable.to_string(),
                                kind: ConfirmKind::DisableFw,
                            }));
                        }
                    }
                    if busy {
                        ui.add(egui::Spinner::new().size(14.0));
                    }
                });
            });

            ui.add_space(space::M);
            components::hairline(ui, p);
            ui.add_space(space::M);

            // Résumé des politiques : lecture immédiate, édition plus bas.
            if let Some(st) = &app.firewall.status {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                    let p_in = st.defaults.incoming.unwrap_or(Policy::Deny);
                    let p_out = st.defaults.outgoing.unwrap_or(Policy::Allow);
                    let p_rt = st.defaults.routed.unwrap_or(Policy::Deny);
                    policy_pill(ui, p, t.policy_incoming, t.policy(p_in), policy_tone(p, p_in));
                    policy_pill(ui, p, t.policy_outgoing, t.policy(p_out), policy_tone(p, p_out));
                    policy_pill(ui, p, t.policy_routed, t.policy(p_rt), policy_tone(p, p_rt));
                    let lvl = if st.logging_on {
                        t.log_level(st.logging_level.unwrap_or(LogLevel::Low))
                    } else {
                        t.log_off
                    };
                    let tone = if st.logging_on { p.warning } else { p.text_secondary };
                    policy_pill(ui, p, t.fw_stat_logging, lvl, tone);
                });
            }

            ui.add_space(space::M);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
                let busy_report = app.pending.values().any(|x| matches!(x, Pending::Report));
                let busy_reload = app.pending.values().any(|x| matches!(x, Pending::Reload));
                if components::styled_button_en(
                    ui,
                    p,
                    &format!("{}  {}", icons::TERMINAL, t.report_btn),
                    components::BtnKind::Ghost,
                    !busy_report,
                )
                .clicked()
                {
                    app.send_op(Pending::Report, Op::UfwReport);
                }
                if components::styled_button_en(
                    ui,
                    p,
                    &format!("{}  {}", icons::RESTART, t.reload_btn),
                    components::BtnKind::Ghost,
                    !busy_reload,
                )
                .on_hover_text(t.reload_rules_tooltip)
                .clicked()
                {
                    app.send_op(Pending::Reload, Op::UfwReload);
                }
                if components::styled_button(ui, p, t.reset_btn, components::BtnKind::Quiet)
                    .clicked()
                {
                    app.modal = Some(Modal::ResetFw {
                        typed: String::new(),
                    });
                }
            });
        },
    );

    if refresh {
        app.send_op(Pending::RefreshStatus, Op::UfwStatus);
        app.send_op(Pending::RefreshUfwService, Op::UfwServiceState);
        app.send_op(Pending::RefreshRules, Op::UfwRules);
    }
}

/// Teinte d'une politique : vert = autorisé, rouge = refusé, ambre = rejeté.
fn policy_tone(p: &Palette, pol: Policy) -> egui::Color32 {
    match pol {
        Policy::Allow => p.success,
        Policy::Deny => p.danger,
        Policy::Reject => p.warning,
    }
}

/// Pastille « libellé · valeur » teintée selon la politique.
fn policy_pill(
    ui: &mut egui::Ui,
    p: &Palette,
    label: &str,
    value: &str,
    tone: egui::Color32,
) {
    Frame::NONE
        .fill(p.soft(tone))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 0.0);
                ui.label(RichText::new(label).size(text::MICRO).color(p.text_secondary));
                ui.label(
                    RichText::new(value)
                        .size(text::MICRO)
                        .color(p.text)
                        .family(egui::FontFamily::Name("Medium".into())),
                );
            });
        });
}

// ─── Chiffres clés ───────────────────────────────────────────────────────────
fn stats_row(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    let rules = &app.firewall.rules;
    let allow = rules
        .iter()
        .filter(|r| r.action == RuleAction::Allow)
        .count();
    let block = rules
        .iter()
        .filter(|r| matches!(r.action, RuleAction::Deny | RuleAction::Reject))
        .count();
    components::tile_grid(ui, p, 150.0, 3, |ui, p, i| match i {
        0 => components::stat_tile(
            ui,
            p,
            icons::RULE,
            p.accent,
            &rules.len().to_string(),
            t.fw_stat_rules,
        ),
        1 => components::stat_tile(
            ui,
            p,
            icons::VERIFIED_USER,
            p.success,
            &allow.to_string(),
            t.fw_stat_allow,
        ),
        _ => components::stat_tile(
            ui,
            p,
            icons::BLOCK,
            p.danger,
            &block.to_string(),
            t.fw_stat_block,
        ),
    });
}

// ─── Politiques par défaut ───────────────────────────────────────────────────
fn policy_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    components::card(ui, p, t.policies_title, |_| {}, |ui| {
        let Some(st) = app.firewall.status.clone() else {
            components::hint(ui, p, t.need_connection_hint);
            return;
        };
        // En fenêtre large, politiques et journalisation se partagent la carte ;
        // en compact, elles s'empilent.
        if ui.available_width() >= 760.0 {
            ui.columns(2, |cols| {
                policy_rows(app, &mut cols[0], p, &st);
                logging_row(app, &mut cols[1], p, &st);
            });
        } else {
            policy_rows(app, ui, p, &st);
            ui.add_space(space::M);
            components::hairline(ui, p);
            ui.add_space(space::M);
            logging_row(app, ui, p, &st);
        }
        ui.add_space(space::S);
        components::hint(ui, p, t.policy_hint);
    });
}

/// Les trois politiques de sens de circulation.
fn policy_rows(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette, st: &FirewallStatus) {
    let t = app.tr;
    for dir in [Direction::Incoming, Direction::Outgoing, Direction::Routed] {
        let dir_label = match dir {
            Direction::Incoming => t.policy_incoming,
            Direction::Outgoing => t.policy_outgoing,
            Direction::Routed => t.policy_routed,
        };
        let current = match dir {
            Direction::Incoming => st.defaults.incoming.unwrap_or(Policy::Deny),
            Direction::Outgoing => st.defaults.outgoing.unwrap_or(Policy::Allow),
            Direction::Routed => st.defaults.routed.unwrap_or(Policy::Deny),
        };
        let mut chosen = current;
        components::labeled_control(ui, p, dir_label, 300.0, |ui| {
            components::segmented(
                ui,
                p,
                &[
                    (Policy::Allow, t.policy_allow),
                    (Policy::Deny, t.policy_deny),
                    (Policy::Reject, t.policy_reject),
                ],
                &mut chosen,
            );
        });
        if dir == Direction::Routed && st.defaults.routed_disabled {
            ui.label(
                RichText::new(t.routed_disabled_note)
                    .size(text::MICRO)
                    .color(p.warning),
            );
        }
        if chosen != current {
            app.send_op(
                Pending::Policy(dir),
                Op::UfwSetDefault {
                    dir,
                    policy: chosen,
                },
            );
        }
        ui.add_space(space::M);
    }
}

/// Niveau de journalisation du pare-feu.
fn logging_row(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette, st: &FirewallStatus) {
    let t = app.tr;
    let current = if st.logging_on {
        st.logging_level.unwrap_or(LogLevel::Low)
    } else {
        LogLevel::Off
    };
    let mut sel = current;
    components::labeled_control(ui, p, t.logging_title, 300.0, |ui| {
        egui::ComboBox::from_id_salt("log-level")
            .selected_text(t.log_level(current))
            .width(ui.available_width().min(220.0))
            .show_ui(ui, |ui| {
                for lvl in [
                    LogLevel::Off,
                    LogLevel::Low,
                    LogLevel::Medium,
                    LogLevel::High,
                    LogLevel::Full,
                ] {
                    ui.selectable_value(&mut sel, lvl, t.log_level(lvl));
                }
            });
    });
    if sel != current {
        app.send_op(Pending::Logging, Op::UfwSetLogging { level: sel });
    }
}

// ─── Journalisation ──────────────────────────────────────────────────────────

// ─── Règles ──────────────────────────────────────────────────────────────────
fn rules_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    let n = app.firewall.rules.len();
    let mut refresh_clicked = false;
    let mut open_clicked = false;
    let mut submit = false;
    let mut cancel = false;

    let filtered: Vec<Rule> = {
        let search = app.firewall.search.to_lowercase();
        app.firewall
            .rules
            .iter()
            .filter(|r| {
                search.is_empty()
                    || r.to.to_lowercase().contains(&search)
                    || r.from.to_lowercase().contains(&search)
                    || t.action(r.action).to_lowercase().contains(&search)
                    || r.comment
                        .as_ref()
                        .map(|c| c.to_lowercase().contains(&search))
                        .unwrap_or(false)
            })
            .cloned()
            .collect()
    };

    components::card(
        ui,
        p,
        &t.rules_with_count(n),
        |ui| {
            if components::icon_button(ui, p, icons::REFRESH, t.refresh_rules_tooltip).clicked() {
                refresh_clicked = true;
            }
            if components::styled_button(
                ui,
                p,
                &format!("{}  {}", icons::ADD, t.add_rule_title),
                components::BtnKind::Primary,
            )
            .clicked()
            {
                open_clicked = true;
            }
        },
        |ui| {
            // ── Composeur de règle (replié par défaut) ──
            if app.add_form_open {
                composer(app, ui, p, &mut submit, &mut cancel);
                ui.add_space(space::M);
            }

            // ── Barre de recherche ──
            ui.horizontal(|ui| {
                let since_w = if app.firewall.rules_at.is_some() {
                    76.0
                } else {
                    0.0
                };
                let w = (ui.available_width() - since_w).max(140.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(w, 0.0),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        components::search_field(
                            ui,
                            p,
                            &mut app.firewall.search,
                            t.search_placeholder,
                        );
                    },
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if let Some(at) = app.firewall.rules_at {
                        components::since_label(ui, p, t, at);
                    }
                });
            });
            ui.add_space(space::S);

            if n == 0 {
                components::empty_state(ui, p, t.rules_empty, t.rules_empty_hint);
                return;
            }
            if filtered.is_empty() {
                components::empty_state(ui, p, t.fw_no_match, t.fw_no_match_hint);
                return;
            }

            let narrow = ui.available_width() < 640.0;
            let mut delete: Option<Rule> = None;
            egui::Grid::new("rules-table")
                .striped(true)
                .num_columns(if narrow { 5 } else { 7 })
                .min_col_width(8.0)
                .spacing(egui::vec2(12.0, 7.0))
                .show(ui, |ui| {
                    ui.label(components::th(p, "#"));
                    ui.label(components::th(p, t.col_action));
                    if !narrow {
                        ui.label(components::th(p, t.col_from));
                    }
                    ui.label(components::th(p, t.col_to_port));
                    ui.label(components::th(p, t.col_dir));
                    if !narrow {
                        ui.label(components::th(p, t.col_comment));
                        ui.label(components::th(p, "IP"));
                    }
                    ui.horizontal(|ui| {
                        ui.label(components::th(p, ""));
                        components::stretch_cell(ui);
                    });
                    ui.end_row();

                    for rule in &filtered {
                        ui.label(
                            RichText::new(rule.number.to_string())
                                .size(text::MONO)
                                .monospace()
                                .color(p.text_secondary),
                        );
                        let (color, label) = match rule.action {
                            RuleAction::Allow => (p.success, t.action_allow),
                            RuleAction::Deny => (p.danger, t.action_deny),
                            RuleAction::Reject => (p.warning, t.action_reject),
                            RuleAction::Limit => (p.accent, t.action_limit),
                        };
                        components::action_badge(ui, p, label, color);
                        if !narrow {
                            ui.label(
                                RichText::new(components::ellipsize(&rule.from, 26)).monospace(),
                            )
                            .on_hover_text(&rule.from);
                        }
                        let port = RichText::new(components::ellipsize(&rule.to, 26)).monospace();
                        if narrow {
                            ui.label(port)
                                .on_hover_text(format!("{}\n← {}", rule.to, rule.from));
                        } else {
                            ui.label(port).on_hover_text(&rule.to);
                        }
                        ui.label(
                            RichText::new(t.rule_dir(rule.dir))
                                .size(text::SMALL)
                                .color(p.text_secondary),
                        );
                        if !narrow {
                            let comment = rule
                                .comment
                                .clone()
                                .unwrap_or_else(|| "—".to_string());
                            ui.label(
                                RichText::new(components::ellipsize(&comment, 22))
                                    .size(text::SMALL)
                                    .color(if rule.comment.is_some() {
                                        p.text_secondary
                                    } else {
                                        p.text_secondary
                                    }),
                            )
                            .on_hover_text(comment);
                            if rule.ipv6 {
                                components::badge(ui, p, "IPv6", p.accent, p.soft(p.accent));
                            } else {
                                components::badge(
                                    ui,
                                    p,
                                    "IPv4",
                                    p.text_secondary,
                                    p.card_hover,
                                );
                            }
                        }
                        let desc = format!(
                            "{} #{} — {} {} ({})",
                            t.btn_delete,
                            rule.number,
                            t.action(rule.action),
                            rule.to,
                            rule.from
                        );
                        if components::icon_danger_button(
                            ui,
                            p,
                            icons::DELETE,
                            &t.delete_rule_tooltip(&desc),
                        )
                        .clicked()
                        {
                            delete = Some(rule.clone());
                        }
                        // Fond de ligne (zébrures) jusqu'au bord de la carte.
                        components::stretch_cell(ui);
                        ui.end_row();
                    }
                });

            if let Some(rule) = delete {
                let mut body = format!(
                    "{} {} — {} ({})",
                    t.col_to_port, rule.to, t.action(rule.action), rule.from
                );
                if let Some(c) = &rule.comment {
                    body.push_str(&format!("\n{c}"));
                }
                app.modal = Some(Modal::Confirm(ConfirmData {
                    title: t.delete_rule_title.to_string(),
                    body,
                    confirm_label: t.btn_delete.to_string(),
                    kind: ConfirmKind::DeleteRule { index: rule.number },
                }));
            }
            ui.add_space(space::S);
            components::hint(ui, p, t.v6_hint);
        },
    );

    if refresh_clicked {
        app.send_op(Pending::RefreshRules, Op::UfwRules);
    }
    if open_clicked {
        app.add_form = crate::app::AddRuleForm::default();
        app.add_form_open = true;
        app.focus_port = true;
        if app.firewall.profiles.is_empty() {
            app.send_op(Pending::RefreshProfiles, Op::UfwAppProfiles);
        }
    }
    if cancel {
        app.add_form_open = false;
    }
    if submit {
        match app.add_form.to_new_rule() {
            Ok(rule) => {
                app.add_form = crate::app::AddRuleForm::default();
                app.send_op(Pending::AddRule, Op::UfwAddRule { rule });
            }
            Err(e) => {
                let msg = app.tr.form_error(e).to_string();
                app.status_msg = msg.clone();
                app.toasts.error(msg);
            }
        }
    }
}

/// Composeur : sens, action, port, protocole, options avancées et aperçu de
/// la commande exacte — au même endroit, pour ne pas disperser l'attention.
fn composer(
    app: &mut GuiApp,
    ui: &mut egui::Ui,
    p: &Palette,
    submit: &mut bool,
    cancel: &mut bool,
) {
    let t = app.tr;
    Frame::NONE
        .fill(p.field)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(11))
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(t.add_rule_title)
                        .size(text::TITLE)
                        .color(p.text)
                        .family(egui::FontFamily::Name("SemiBold".into())),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if components::icon_button(ui, p, icons::CLOSE, t.close_btn).clicked() {
                        *cancel = true;
                    }
                });
            });
            ui.add_space(space::S);

            components::labeled_control(ui, p, t.action_label, 520.0, |ui| {
                let mut action = app.add_form.action;
                components::segmented(
                    ui,
                    p,
                    &[
                        (RuleAction::Allow, t.action_allow),
                        (RuleAction::Deny, t.action_deny),
                        (RuleAction::Reject, t.action_reject),
                        (RuleAction::Limit, t.action_limit),
                    ],
                    &mut action,
                );
                app.add_form.action = action;
            });
            components::labeled_control(ui, p, t.dir_label, 520.0, |ui| {
                let mut dir = app.add_form.dir;
                components::segmented(
                    ui,
                    p,
                    &[
                        (NewRuleDir::In, t.dir_inbound),
                        (NewRuleDir::Out, t.dir_outbound),
                    ],
                    &mut dir,
                );
                app.add_form.dir = dir;
            });

            // Port + protocole + profil d'application.
            ui.add_space(space::XS);
            ui.horizontal(|ui| {
                let proto_w = 170.0;
                let port_w = (ui.available_width() - proto_w - 8.0).max(120.0);
                let resp = components::field_sized(
                    ui,
                    p,
                    t.field_port_service,
                    &mut app.add_form.port,
                    t.hint_port,
                    port_w,
                );
                if app.focus_port {
                    resp.request_focus();
                    app.focus_port = false;
                }
                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    *submit = true;
                }
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(t.proto_label)
                            .size(text::SMALL)
                            .color(p.text_secondary)
                            .family(egui::FontFamily::Name("Medium".into())),
                    );
                    let mut proto = app.add_form.proto;
                    components::segmented(
                        ui,
                        p,
                        &[
                            (Proto::Tcp, t.proto_tcp),
                            (Proto::Udp, t.proto_udp),
                            (Proto::Any, t.proto_any),
                        ],
                        &mut proto,
                    );
                    app.add_form.proto = proto;
                });
            });
            if !app.firewall.profiles.is_empty() {
                ui.add_space(space::XS);
                let mut use_profile = app.add_form.use_profile;
                if ui.checkbox(&mut use_profile, t.use_profile_cb).changed() {
                    app.add_form.use_profile = use_profile;
                }
                if use_profile {
                    let sel = app
                        .add_form
                        .profile
                        .clone()
                        .unwrap_or_else(|| app.firewall.profiles.first().cloned().unwrap_or_default());
                    let mut chosen = sel.clone();
                    egui::ComboBox::from_id_salt("rule-profile")
                        .selected_text(&sel)
                        .width(ui.available_width())
                        .show_ui(ui, |ui| {
                            for prof in &app.firewall.profiles {
                                ui.selectable_value(&mut chosen, prof.clone(), prof);
                            }
                        });
                    app.add_form.profile = Some(chosen);
                }
            }

            // Options avancées.
            let id = ui.make_persistent_id("rule-adv-open");
            let mut adv = ui.data(|d| d.get_temp::<bool>(id).unwrap_or(false));
            let chevron = if adv {
                icons::EXPAND_LESS
            } else {
                icons::EXPAND_MORE
            };
            if components::link_button(ui, p, &format!("{chevron}  {}", t.adv_options)).clicked() {
                adv = !adv;
                ui.data_mut(|d| d.insert_temp(id, adv));
            }
            if adv {
                ui.add_space(space::XS);
                let f = &mut app.add_form;
                components::field(ui, p, t.field_source, &mut f.from_addr, t.hint_source);
                components::field(ui, p, t.field_dest, &mut f.to_addr, t.hint_dest);
                ui.horizontal(|ui| {
                    let w = 120.0;
                    components::field_sized(ui, p, t.field_insert, &mut f.insert_at, t.hint_insert, w);
                    let cw = ui.available_width();
                    components::field_sized(ui, p, t.field_comment, &mut f.comment, t.hint_comment, cw);
                });
            }

            // Aperçu de la commande exacte.
            ui.add_space(space::M);
            ui.label(
                RichText::new(t.preview_title)
                    .size(text::MICRO)
                    .color(p.text_secondary),
            );
            ui.add_space(2.0);
            let empty = app.add_form.port.trim().is_empty() && !app.add_form.use_profile;
            let error = app.add_form.to_new_rule().err();
            let valid = error.is_none();
            let preview = app.add_form.preview(t);
            Frame::NONE
                .fill(p.card)
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(icons::icon_sz(
                            if valid {
                                icons::TERMINAL
                            } else {
                                icons::WARNING
                            },
                            14.0,
                            if valid { p.text_secondary } else { p.danger },
                        ));
                        let color = if valid { p.text } else { p.danger };
                        ui.label(
                            RichText::new(crate::components::ellipsize(&preview, 120))
                                .monospace()
                                .size(text::MONO)
                                .color(color),
                        )
                        .on_hover_text(&preview);
                    });
                });
            if let (Some(e), false) = (error, empty) {
                ui.label(
                    RichText::new(t.form_error(e))
                        .size(text::MICRO)
                        .color(p.danger),
                );
            }

            ui.add_space(space::M);
            ui.horizontal(|ui| {
                let enabled = valid;
                if ui.available_width() >= 360.0 {
                    if components::styled_button(ui, p, t.cancel_btn, components::BtnKind::Ghost)
                        .clicked()
                    {
                        *cancel = true;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if components::styled_button_en(
                            ui,
                            p,
                            &format!("{}  {}", icons::CHECK, t.btn_add),
                            components::BtnKind::Primary,
                            enabled,
                        )
                        .clicked()
                        {
                            *submit = true;
                        }
                    });
                } else {
                    if components::styled_button_fill(
                        ui,
                        p,
                        &format!("{}  {}", icons::CHECK, t.btn_add),
                        components::BtnKind::Primary,
                        enabled,
                    )
                    .clicked()
                    {
                        *submit = true;
                    }
                    ui.add_space(space::XS);
                    if components::styled_button(ui, p, t.cancel_btn, components::BtnKind::Ghost)
                        .clicked()
                    {
                        *cancel = true;
                    }
                }
            });
        });
}

// ─── Détails techniques ──────────────────────────────────────────────────────
fn technical_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    let id = ui.make_persistent_id("fw-technical-open");
    let mut open = ui.data(|d| d.get_temp::<bool>(id).unwrap_or(false));
    let chevron = if open {
        icons::EXPAND_LESS
    } else {
        icons::EXPAND_MORE
    };
    if components::link_button(ui, p, &format!("{chevron}  {}", t.fw_technical)).clicked() {
        open = !open;
        ui.data_mut(|d| d.insert_temp(id, open));
    }
    if !open {
        return;
    }
    ui.add_space(space::S);

    let mut service_action: Option<ServiceAction> = None;
    let mut show_raw = app.firewall.show_raw;
    components::card(ui, p, t.fw_service_title, |_| {}, |ui| {
        let (state, at) = match &app.firewall.ufw_service {
            Some(s) => (s.clone(), app.firewall.ufw_service_at),
            None => (t.state_unknown.to_string(), None),
        };
        let active = state == "active";
        let color = if active {
            p.success
        } else if state == "failed" {
            p.danger
        } else {
            p.text_secondary
        };
        ui.horizontal_wrapped(|ui| {
            components::status_dot(ui, color, 8.0);
            ui.label(
                RichText::new("ufw")
                    .monospace()
                    .family(egui::FontFamily::Name("MonoMedium".into())),
            );
            components::tag(ui, p, &state, color);
            if let Some(at) = at {
                components::since_label(ui, p, t, at);
            }
        });
        ui.add_space(space::S);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            if active {
                if components::ghost_button(ui, p, t.btn_restart).clicked() {
                    service_action = Some(ServiceAction::Restart);
                }
                if components::ghost_button(ui, p, t.btn_stop).clicked() {
                    service_action = Some(ServiceAction::Stop);
                }
            } else if app.firewall.ufw_service.is_some()
                && components::primary_button(ui, p, t.btn_start).clicked()
            {
                service_action = Some(ServiceAction::Start);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .selectable_label(show_raw, RichText::new(t.show_raw_toggle).size(text::SMALL))
                    .clicked()
                {
                    show_raw = !show_raw;
                }
            });
        });
        ui.add_space(space::S);
        components::hint(ui, p, t.ufw_service_hint);
        if show_raw {
            ui.add_space(space::S);
            components::section(ui, p, t.fw_raw_title);
            Frame::NONE
                .fill(p.field)
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::same(10))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    if let Some(s) = &app.firewall.status {
                        egui::ScrollArea::both()
                            .id_salt("ufw-raw")
                            .max_height(220.0)
                            .min_scrolled_height(120.0)
                            .show(ui, |ui| {
                                ui.monospace(&s.raw);
                            });
                    }
                });
        }
    });
    app.firewall.show_raw = show_raw;

    if let Some(action) = service_action {
        match action {
            ServiceAction::Start | ServiceAction::ReloadOrRestart => {
                app.send_op(
                    Pending::UfwService(action),
                    Op::UfwServiceAction { action },
                );
            }
            ServiceAction::Restart => {
                app.modal = Some(Modal::Confirm(ConfirmData {
                    title: t.ufw_restart_title.to_string(),
                    body: t.ufw_restart_body.to_string(),
                    confirm_label: t.btn_restart.to_string(),
                    kind: ConfirmKind::RestartUfw,
                }));
            }
            ServiceAction::Stop => {
                app.modal = Some(Modal::Confirm(ConfirmData {
                    title: t.ufw_stop_title.to_string(),
                    body: t.ufw_stop_body.to_string(),
                    confirm_label: t.btn_stop.to_string(),
                    kind: ConfirmKind::StopUfw,
                }));
            }
        }
    }
}
