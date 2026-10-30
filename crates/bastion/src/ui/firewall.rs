//! Écran Pare-feu : état, politiques par défaut, journalisation, règles.

use crate::app::{ConfirmData, ConfirmKind, GuiApp, Modal, Pending};
use crate::components;
use crate::theme::Palette;
use bastion_core::model::*;
use bastion_core::op::{Op, ServiceAction};
use egui::{Align, Layout, RichText};

pub fn render(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    components::screen_header(ui, p, t.nav_firewall, t.fw_subtitle);
    ui.add_space(6.0);

    if app.phase != crate::app::ConnPhase::Connected {
        components::empty_state(ui, p, t.status_disconnected, t.need_connection_hint);
        return;
    }

    // Cartes empilées verticalement : état, puis politiques par défaut,
    // puis journalisation, chacune en pleine largeur.
    status_card(app, ui, p);
    ui.add_space(12.0);
    services_card(app, ui, p);
    ui.add_space(12.0);
    policy_card(app, ui, p);
    ui.add_space(12.0);
    logging_card(app, ui, p);
    ui.add_space(12.0);
    rules_card(app, ui, p);
    ui.add_space(12.0);

    // Actions rares, en bas.
    ui.horizontal(|ui| {
        let busy_report = app.pending.values().any(|x| matches!(x, Pending::Report));
        if ui
            .add_enabled(
                !busy_report,
                egui::Button::new(components::btn_icon_text(crate::icons::TERMINAL, t.report_btn)),
            )
            .clicked()
        {
            app.send_op(Pending::Report, Op::UfwReport);
        }
        if components::danger_button(ui, p, t.reset_btn).clicked() {
            app.modal = Some(Modal::ResetFw { typed: String::new() });
        }
        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
            let busy_reload = app.pending.values().any(|x| matches!(x, Pending::Reload));
            if ui
                .add_enabled(
                    !busy_reload,
                    egui::Button::new(components::btn_icon_text(crate::icons::RESTART, t.reload_btn)),
                )
                .on_hover_text(t.reload_rules_tooltip)
                .clicked()
            {
                app.send_op(Pending::Reload, Op::UfwReload);
            }
        });
    });
}

// ─── Carte état ──────────────────────────────────────────────────────────────
fn status_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    components::card_body(ui, p, |ui| {
        let active = app.firewall.status.as_ref().map(|s| s.active).unwrap_or(false);
        let version = app.probe.0.clone().unwrap_or_default();
        let busy = app
            .pending
            .values()
            .any(|x| matches!(x, Pending::Enable | Pending::Disable | Pending::RefreshStatus));

        components::status_line(
            ui,
            p,
            if active { p.success } else { p.danger },
            if active { t.fw_state_active } else { t.fw_state_inactive },
            None,
        );
        if !version.is_empty() {
            components::hint(ui, p, &version);
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let on = components::switch(ui, p, active, active.then_some(p.success));
            if on != active && !busy {
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

        // Service systemd ufw : état + démarrer / redémarrer / arrêter.
        ui.add_space(10.0);
        let svc = app
            .firewall
            .ufw_service
            .clone()
            .unwrap_or_else(|| t.state_unknown.to_string());
        let svc_active = svc == "active";
        let svc_col = if svc_active {
            p.success
        } else if svc == "failed" {
            p.danger
        } else {
            p.text_secondary
        };
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
            ui.painter().circle_filled(rect.center(), 5.0, svc_col);
            ui.label(
                RichText::new(format!("{} · {}", t.service_label, svc))
                    .size(12.5)
                    .color(p.text_secondary),
            );
        });
        ui.add_space(6.0);
        let busy_svc = app
            .pending
            .values()
            .any(|x| matches!(x, Pending::UfwService(_)));
        ui.horizontal(|ui| {
            if svc_active {
                if components::ghost_button(ui, p, t.btn_restart).clicked() {
                    app.modal = Some(Modal::Confirm(ConfirmData {
                        title: t.ufw_restart_title.to_string(),
                        body: t.ufw_restart_body.to_string(),
                        confirm_label: t.btn_restart.to_string(),
                        kind: ConfirmKind::RestartUfw,
                    }));
                }
                if components::ghost_button(ui, p, t.btn_stop).clicked() {
                    app.modal = Some(Modal::Confirm(ConfirmData {
                        title: t.ufw_stop_title.to_string(),
                        body: t.ufw_stop_body.to_string(),
                        confirm_label: t.btn_stop.to_string(),
                        kind: ConfirmKind::StopUfw,
                    }));
                }
            } else if !busy_svc && app.firewall.ufw_service.is_some() {
                if components::primary_button(ui, p, t.btn_start).clicked() {
                    app.send_op(
                        Pending::UfwService(ServiceAction::Start),
                        Op::UfwServiceAction { action: ServiceAction::Start },
                    );
                }
            }
            if busy_svc {
                ui.add(egui::Spinner::new().size(14.0));
            }
        });
        ui.add_space(6.0);
        components::hint(ui, p, t.ufw_service_hint);

        ui.add_space(2.0);
        if components::styled_button(ui, p, t.show_raw_toggle, components::BtnKind::Soft).clicked() {
            app.firewall.show_raw = !app.firewall.show_raw;
        }
        if app.firewall.show_raw {
            if let Some(s) = &app.firewall.status {
                egui::ScrollArea::vertical()
                    .id_salt("ufw-raw")
                    .max_height(140.0)
                    .show(ui, |ui| ui.monospace(&s.raw));
            }
        }
    });
}

// ─── Carte services pare-feu actifs ─────────────────────────────────────────
fn services_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    let mut refresh_clicked = false;

    // États relevés avant le rendu (emprunts isolés).
    let rows: Vec<(&str, String, Option<std::time::Instant>)> = vec![
        (
            "ufw",
            app.firewall
                .ufw_service
                .clone()
                .unwrap_or_else(|| t.state_unknown.to_string()),
            app.firewall.ufw_service_at,
        ),
        (
            "fail2ban",
            app.f2b
                .service
                .clone()
                .unwrap_or_else(|| t.state_unknown.to_string()),
            app.f2b.service_at,
        ),
    ];

    components::card(
        ui,
        p,
        t.fw_services_title,
        |ui| {
            if components::icon_button(ui, p, crate::icons::REFRESH, t.tooltip_refresh).clicked() {
                refresh_clicked = true;
            }
        },
        |ui| {
            if app.phase != crate::app::ConnPhase::Connected {
                components::hint(ui, p, t.need_connection_hint);
                return;
            }
            for (name, state, at) in &rows {
                ui.horizontal(|ui| {
                    let col = if state == "active" {
                        p.success
                    } else if state == "failed" {
                        p.danger
                    } else {
                        p.text_secondary
                    };
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 5.0, col);
                    ui.label(RichText::new(*name).monospace());
                    let bg = crate::components::blend(
                        col,
                        ui.style().visuals.panel_fill,
                        0.18,
                    );
                    components::badge(ui, p, state, col, bg);
                    if let Some(at) = at {
                        components::since_label(ui, p, t, *at);
                    }
                });
            }
        },
    );

    if refresh_clicked {
        app.send_op(Pending::RefreshUfwService, Op::UfwServiceState);
        app.send_op(Pending::RefreshService, Op::ServiceState);
    }
}

// ─── Carte politiques ────────────────────────────────────────────────────────
fn policy_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    components::card_body(ui, p, |ui| {
        ui.label(
            RichText::new(t.policies_title.to_uppercase())
                .size(11.5)
                .weak()
                .family(egui::FontFamily::Name("Medium".into())),
        );
        ui.add_space(6.0);
        let Some(st) = app.firewall.status.clone() else {
            return;
        };
        for dir in [Direction::Incoming, Direction::Outgoing, Direction::Routed] {
            ui.horizontal(|ui| {
                let dir_label = match dir {
                    Direction::Incoming => t.policy_incoming,
                    Direction::Outgoing => t.policy_outgoing,
                    Direction::Routed => t.policy_routed,
                };
                ui.label(RichText::new(dir_label).size(12.5).color(p.text_secondary));
                let current = match dir {
                    Direction::Incoming => st.defaults.incoming.unwrap_or(Policy::Deny),
                    Direction::Outgoing => st.defaults.outgoing.unwrap_or(Policy::Allow),
                    Direction::Routed => st.defaults.routed.unwrap_or(Policy::Deny),
                };
                let mut sel = current;
                egui::ComboBox::from_id_salt(format!("policy-{dir:?}"))
                    .selected_text(t.policy(current))
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        for pol in [Policy::Allow, Policy::Deny, Policy::Reject] {
                            ui.selectable_value(&mut sel, pol, t.policy(pol));
                        }
                    });
                if dir == Direction::Routed && st.defaults.routed_disabled {
                    ui.label(
                        RichText::new(t.routed_disabled_note)
                            .size(10.5)
                            .color(p.warning),
                    );
                }
                if sel != current {
                    app.send_op(Pending::Policy(dir), Op::UfwSetDefault { dir, policy: sel });
                }
            });
        }
        ui.add_space(2.0);
        components::hint(ui, p, t.policy_hint);
    });
}

// ─── Carte journalisation ────────────────────────────────────────────────────
fn logging_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    components::card_body(ui, p, |ui| {
        ui.label(
            RichText::new(t.logging_title.to_uppercase())
                .size(11.5)
                .weak()
                .family(egui::FontFamily::Name("Medium".into())),
        );
        ui.add_space(6.0);
        let Some(st) = app.firewall.status.clone() else {
            return;
        };
        let current = if st.logging_on {
            st.logging_level.unwrap_or(LogLevel::Low)
        } else {
            LogLevel::Off
        };
        let mut sel = current;
        egui::ComboBox::from_id_salt("log-level")
            .selected_text(t.log_level(current))
            .width(150.0)
            .show_ui(ui, |ui| {
                for lvl in [LogLevel::Off, LogLevel::Low, LogLevel::Medium, LogLevel::High, LogLevel::Full] {
                    ui.selectable_value(&mut sel, lvl, t.log_level(lvl));
                }
            });
        if sel != current {
            app.send_op(Pending::Logging, Op::UfwSetLogging { level: sel });
        }
        ui.add_space(4.0);
        components::hint(ui, p, t.logging_hint);
    });
}

// ─── Carte règles ────────────────────────────────────────────────────────────
fn rules_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    let n = app.firewall.rules.len();
    let mut refresh_clicked = false;
    let mut submit = false;

    components::card(
        ui,
        p,
        &t.rules_with_count(n),
        |ui| {
            if components::icon_button(ui, p, crate::icons::REFRESH, t.refresh_rules_tooltip).clicked() {
                refresh_clicked = true;
            }
        },
        |ui| {
            // ── Formulaire d'ajout de règle, inline (pas de popin) ──
            ui.label(
                RichText::new(t.add_rule_title)
                    .size(13.0)
                    .family(egui::FontFamily::Name("SemiBold".into())),
            );
            ui.add_space(4.0);
            ui.label(RichText::new(t.field_port_service).size(12.0).color(p.text_secondary));
            ui.horizontal(|ui| {
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut app.add_form.port)
                        .hint_text(t.hint_port)
                        .desired_width((ui.available_width() - 130.0).max(160.0))
                        .font(egui::TextStyle::Monospace),
                );
                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    submit = true;
                }
                if components::primary_button_small(ui, p, crate::icons::ADD, t.btn_add)
                    .on_hover_text(t.hint_port)
                    .clicked()
                {
                    submit = true;
                }
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(10.0, 8.0);
                ui.label(RichText::new(t.proto_label).size(12.0).color(p.text_secondary));
                let mut proto = app.add_form.proto;
                components::segmented(
                    ui,
                    p,
                    &[(Proto::Tcp, t.proto_tcp), (Proto::Udp, t.proto_udp), (Proto::Any, t.proto_any)],
                    &mut proto,
                );
                app.add_form.proto = proto;
            });
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(10.0, 8.0);
                ui.label(RichText::new(t.dir_label).size(12.0).color(p.text_secondary));
                let mut dir = app.add_form.dir;
                components::segmented(
                    ui,
                    p,
                    &[(NewRuleDir::In, t.dir_inbound), (NewRuleDir::Out, t.dir_outbound)],
                    &mut dir,
                );
                app.add_form.dir = dir;
            });
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(10.0, 8.0);
                ui.label(RichText::new(t.action_label).size(12.0).color(p.text_secondary));
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
            ui.add_space(4.0);

            // Profil d'application UFW, s'il y en a.
            if !app.firewall.profiles.is_empty() {
                ui.checkbox(&mut app.add_form.use_profile, t.use_profile_cb);
                if app.add_form.use_profile {
                    let sel = app
                        .add_form
                        .profile
                        .clone()
                        .unwrap_or_else(|| app.firewall.profiles.first().cloned().unwrap_or_default());
                    let mut chosen = sel.clone();
                    egui::ComboBox::from_id_salt("rule-profile")
                        .selected_text(&sel)
                        .width(240.0)
                        .show_ui(ui, |ui| {
                            for prof in &app.firewall.profiles {
                                ui.selectable_value(&mut chosen, prof.clone(), prof);
                            }
                        });
                    app.add_form.profile = Some(chosen);
                }
            }

            // Options avancées repliables.
            egui::CollapsingHeader::new(
                RichText::new(format!("{}  {}", crate::icons::EXPAND_MORE, t.adv_options))
                    .size(12.5)
                    .family(egui::FontFamily::Name("Medium".into())),
            )
            .id_salt("rule-adv")
            .show(ui, |ui| {
                let f = &mut app.add_form;
                components::field(ui, p, t.field_source, &mut f.from_addr, t.hint_source);
                components::field(ui, p, t.field_dest, &mut f.to_addr, t.hint_dest);
                components::field(ui, p, t.field_insert, &mut f.insert_at, t.hint_insert);
                components::field(ui, p, t.field_comment, &mut f.comment, t.hint_comment);
            });
            ui.add_space(6.0);

            // Aperçu de la commande.
            let preview = app.add_form.preview(t);
            ui.label(RichText::new(t.preview_label).size(11.5).color(p.text_secondary));
            ui.label(RichText::new(ellipsize(&preview, 110)).monospace())
                .on_hover_text(&preview);
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label(crate::icons::icon(crate::icons::SEARCH));
                ui.add(
                    egui::TextEdit::singleline(&mut app.firewall.search)
                        .hint_text(t.search_placeholder)
                        .desired_width(280.0),
                );
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    if let Some(at) = app.firewall.rules_at {
                        components::since_label(ui, p, t, at);
                    }
                });
            });
            ui.add_space(8.0);

            if n == 0 {
                components::empty_state(ui, p, t.rules_empty, t.rules_empty_hint);
                return;
            }

            let search = app.firewall.search.to_lowercase();
            let rules: Vec<Rule> = app
                .firewall
                .rules
                .iter()
                .filter(|r| {
                    search.is_empty()
                        || r.to.to_lowercase().contains(&search)
                        || r.from.to_lowercase().contains(&search)
                        || r.comment
                            .as_ref()
                            .map(|c| c.to_lowercase().contains(&search))
                            .unwrap_or(false)
                })
                .cloned()
                .collect();

            // Hauteur généreuse : au moins 20 règles visibles, jusqu'à ~50
            // selon le nombre de règles filtrées.
            let list_h = (30.0 + 30.0 * rules.len() as f32).clamp(630.0, 1530.0);
            egui::ScrollArea::vertical()
                .max_height(list_h)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    egui::Grid::new("rules-table")
                        .striped(true)
                        .num_columns(7)
                        .min_col_width(10.0)
                        .spacing(egui::vec2(14.0, 8.0))
                        .show(ui, |ui| {
                            ui.label(header(p, "#"));
                            ui.label(header(p, t.col_action));
                            ui.label(header(p, t.col_from));
                            ui.label(header(p, t.col_to_port));
                            ui.label(header(p, t.col_dir));
                            ui.label(header(p, t.col_comment));
                            ui.label(header(p, ""));
                            ui.end_row();

                            for rule in &rules {
                                ui.label(RichText::new(rule.number.to_string()).weak());
                                let (color, label) = match rule.action {
                                    RuleAction::Allow => (p.success, t.action_allow),
                                    RuleAction::Deny => (p.danger, t.action_deny),
                                    RuleAction::Reject => (p.warning, t.action_reject),
                                    RuleAction::Limit => (p.accent, t.action_limit),
                                };
                                components::action_badge(ui, p, label, color);
                                ui.label(RichText::new(ellipsize(&rule.from, 30)).monospace())
                                    .on_hover_text(&rule.from);
                                ui.label(RichText::new(ellipsize(&rule.to, 30)).monospace())
                                    .on_hover_text(&rule.to);
                                let mut dir_txt = t.rule_dir(rule.dir).to_string();
                                if rule.ipv6 {
                                    dir_txt.push_str(" · v6");
                                }
                                ui.label(RichText::new(dir_txt).size(12.0).color(p.text_secondary));
                                match &rule.comment {
                                    Some(c) => {
                                        ui.label(RichText::new(ellipsize(c, 45)).size(12.5))
                                            .on_hover_text(c);
                                    }
                                    None => {
                                        ui.label(RichText::new("—").weak());
                                    }
                                };
                                let desc = format!(
                                    "#{} — {} {} ({})",
                                    rule.number,
                                    t.action(rule.action),
                                    rule.to,
                                    rule.from
                                );
                                if components::icon_button(
                                    ui,
                                    p,
                                    crate::icons::DELETE,
                                    &t.delete_rule_tooltip(&desc),
                                )
                                .clicked()
                                {
                                    app.modal = Some(Modal::Confirm(ConfirmData {
                                        title: t.delete_rule_title.to_string(),
                                        body: desc,
                                        confirm_label: t.btn_delete.to_string(),
                                        kind: ConfirmKind::DeleteRule { index: rule.number },
                                    }));
                                }
                                ui.end_row();
                            }
                        });
                });
            ui.add_space(2.0);
            components::hint(ui, p, t.mono_hint);
            ui.label(RichText::new(t.v6_hint).size(12.5).color(p.text_secondary));
        },
    );

    if refresh_clicked {
        app.send_op(Pending::RefreshRules, Op::UfwRules);
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

fn header(p: &Palette, s: &str) -> RichText {
    RichText::new(s)
        .size(11.5)
        .weak()
        .family(egui::FontFamily::Name("Medium".into()))
        .color(p.text_secondary)
}

/// Coupe proprement une chaîne (nombre de caractères) avec ellipse.
pub fn ellipsize(s: &str, max: usize) -> String {
    components::ellipsize(s, max)
}