//! Écran Pare-feu : état, services, politiques, journalisation, ports actifs,
//! aperçu de commande et règles. Les cartes forment une grille responsive
//! (côte à côte en fenêtre large, empilées en fenêtre étroite), avec un
//! en-tête uniforme.

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
        components::card_body(ui, p, |ui| {
            components::empty_state(ui, p, t.status_disconnected, t.need_connection_hint);
        });
        return;
    }

    // Grille de tuiles responsive (mobile-first) : côte à côte dès que la
    // largeur le permet, empilées en fenêtre étroite.
    components::tile_grid(ui, p, 340.0, 2, |ui, p, i| match i {
        0 => status_card(app, ui, p),
        _ => services_card(app, ui, p),
    });
    ui.add_space(12.0);
    components::tile_grid(ui, p, 300.0, 2, |ui, p, i| match i {
        0 => policy_card(app, ui, p),
        _ => logging_card(app, ui, p),
    });
    ui.add_space(12.0);
    components::tile_grid(ui, p, 420.0, 2, |ui, p, i| match i {
        0 => ports_card(app, ui, p),
        _ => preview_card(app, ui, p),
    });
    ui.add_space(12.0);
    rules_card(app, ui, p);
    ui.add_space(12.0);

    // Actions rares, en bas, en boutons uikit.
    let busy_report = app.pending.values().any(|x| matches!(x, Pending::Report));
    let busy_reload = app.pending.values().any(|x| matches!(x, Pending::Reload));
    // En fenêtre large, « Recharger » reste aligné à droite ; en fenêtre
    // étroite les trois boutons se replient à la ligne.
    if ui.available_width() >= 620.0 {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            let mut report = false;
            let mut reset = false;
            let mut reload = false;
            if components::styled_button_en(
                ui,
                p,
                &format!("{}  {}", crate::icons::TERMINAL, t.report_btn),
                components::BtnKind::GhostOnBg,
                !busy_report,
            )
            .clicked()
            {
                report = true;
            }
            if components::danger_button(ui, p, t.reset_btn).clicked() {
                reset = true;
            }
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                if components::styled_button_en(
                    ui,
                    p,
                    &format!("{}  {}", crate::icons::RESTART, t.reload_btn),
                    components::BtnKind::GhostOnBg,
                    !busy_reload,
                )
                .on_hover_text(t.reload_rules_tooltip)
                .clicked()
                {
                    reload = true;
                }
            });
            if report {
                app.send_op(Pending::Report, Op::UfwReport);
            }
            if reset {
                app.modal = Some(Modal::ResetFw { typed: String::new() });
            }
            if reload {
                app.send_op(Pending::Reload, Op::UfwReload);
            }
        });
    } else {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            if components::styled_button_en(
                ui,
                p,
                &format!("{}  {}", crate::icons::TERMINAL, t.report_btn),
                components::BtnKind::GhostOnBg,
                !busy_report,
            )
            .clicked()
            {
                app.send_op(Pending::Report, Op::UfwReport);
            }
            if components::danger_button(ui, p, t.reset_btn).clicked() {
                app.modal = Some(Modal::ResetFw { typed: String::new() });
            }
            if components::styled_button_en(
                ui,
                p,
                &format!("{}  {}", crate::icons::RESTART, t.reload_btn),
                components::BtnKind::GhostOnBg,
                !busy_reload,
            )
            .on_hover_text(t.reload_rules_tooltip)
            .clicked()
            {
                app.send_op(Pending::Reload, Op::UfwReload);
            }
        });
    }
}

// ─── Carte état du pare-feu ──────────────────────────────────────────────────
fn status_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    let active = app.firewall.status.as_ref().map(|s| s.active).unwrap_or(false);
    let version = app.probe.0.clone().unwrap_or_default();
    let svc = app
        .firewall
        .ufw_service
        .clone()
        .unwrap_or_else(|| t.state_unknown.to_string());
    let svc_active = svc == "active";
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
            if components::icon_button(ui, p, crate::icons::REFRESH, t.tooltip_refresh).clicked() {
                refresh = true;
            }
        },
        |ui| {
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

            // Interrupteur d'activation + indicateur d'opération en cours.
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
            ui.add_space(10.0);

            // Service systemd ufw : état + démarrer / redémarrer / arrêter.
            let svc_col = if svc_active {
                p.success
            } else if svc == "failed" {
                p.danger
            } else {
                p.text_secondary
            };
            ui.horizontal(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                ui.painter().circle_filled(rect.center(), 5.0, svc_col);
                ui.label(
                    RichText::new(format!("{} · {}", t.service_label, svc))
                        .size(12.5)
                        .color(p.text_secondary),
                );
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
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
                } else if app.firewall.ufw_service.is_some() {
                    if components::primary_button(ui, p, t.btn_start).clicked() {
                        app.send_op(
                            Pending::UfwService(ServiceAction::Start),
                            Op::UfwServiceAction { action: ServiceAction::Start },
                        );
                    }
                }
            });
            ui.add_space(6.0);
            components::hint(ui, p, t.ufw_service_hint);

            ui.add_space(2.0);
            if components::styled_button(ui, p, t.show_raw_toggle, components::BtnKind::Soft)
                .clicked()
            {
                app.firewall.show_raw = !app.firewall.show_raw;
            }
            if app.firewall.show_raw {
                if let Some(s) = &app.firewall.status {
                    egui::ScrollArea::vertical()
                        .id_salt("ufw-raw")
                        .max_height(140.0)
                        .min_scrolled_height(140.0)
                        .show(ui, |ui| ui.monospace(&s.raw));
                }
            }
        },
    );

    if refresh {
        app.send_op(Pending::RefreshStatus, Op::UfwStatus);
        app.send_op(Pending::RefreshUfwService, Op::UfwServiceState);
    }
}

// ─── Carte services pare-feu ─────────────────────────────────────────────────
fn services_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    let mut refresh_clicked = false;

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
                    let bg = crate::components::blend(col, ui.style().visuals.panel_fill, 0.18);
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

// ─── Carte politiques par défaut ─────────────────────────────────────────────
fn policy_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    components::card(ui, p, t.policies_title, |_| {}, |ui| {
        let Some(st) = app.firewall.status.clone() else {
            return;
        };
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
            // Paramètre responsive : libellé + sélecteur sur la même ligne
            // si la place suffit, empilés sinon.
            let mut chosen = current;
            components::labeled_control(ui, p, dir_label, 360.0, |ui| {
                egui::ComboBox::from_id_salt(format!("policy-{dir:?}"))
                    .selected_text(t.policy(current))
                    .width(130.0)
                    .show_ui(ui, |ui| {
                        for pol in [Policy::Allow, Policy::Deny, Policy::Reject] {
                            ui.selectable_value(&mut chosen, pol, t.policy(pol));
                        }
                    });
            });
            if dir == Direction::Routed && st.defaults.routed_disabled {
                ui.label(
                    RichText::new(t.routed_disabled_note)
                        .size(10.5)
                        .color(p.warning),
                );
            }
            if chosen != current {
                app.send_op(Pending::Policy(dir), Op::UfwSetDefault { dir, policy: chosen });
            }
        }
        ui.add_space(2.0);
        components::hint(ui, p, t.policy_hint);
    });
}

// ─── Carte journalisation ────────────────────────────────────────────────────
fn logging_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    components::card(ui, p, t.logging_title, |_| {}, |ui| {
        let Some(st) = app.firewall.status.clone() else {
            return;
        };
        let current = if st.logging_on {
            st.logging_level.unwrap_or(LogLevel::Low)
        } else {
            LogLevel::Off
        };
        let mut sel = current;
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("log-level")
                .selected_text(t.log_level(current))
                .width(150.0)
                .show_ui(ui, |ui| {
                    for lvl in [LogLevel::Off, LogLevel::Low, LogLevel::Medium, LogLevel::High, LogLevel::Full] {
                        ui.selectable_value(&mut sel, lvl, t.log_level(lvl));
                    }
                });
        });
        if sel != current {
            app.send_op(Pending::Logging, Op::UfwSetLogging { level: sel });
        }
        ui.add_space(4.0);
        components::hint(ui, p, t.logging_hint);
    });
}

// ─── Carte ports actifs ──────────────────────────────────────────────────────
fn ports_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    // Ports/services ouverts, dérivés des règles (colonne « To » ≠ Anywhere),
    // regroupés par (port, action, direction) avec numéros de règles et v6.
    let mut specs: Vec<(String, RuleAction, RuleDir, Vec<String>, bool)> = Vec::new();
    for r in &app.firewall.rules {
        if r.to.is_empty() || r.to.eq_ignore_ascii_case("anywhere") {
            continue;
        }
        let key = (r.to.clone(), r.action, r.dir);
        match specs
            .iter_mut()
            .find(|(to, a, d, _, _)| (to.clone(), *a, *d) == key)
        {
            Some((_, _, _, nums, v6)) => {
                nums.push(r.number.to_string());
                *v6 = *v6 || r.ipv6;
            }
            None => specs.push((
                r.to.clone(),
                r.action,
                r.dir,
                vec![r.number.to_string()],
                r.ipv6,
            )),
        }
    }

    components::card(ui, p, t.ports_active_title, |_| {}, |ui| {
        if specs.is_empty() {
            components::hint(ui, p, t.ports_empty_hint);
            return;
        }
        // Tableau responsive : en fenêtre étroite, les colonnes d'adresse
        // IP et de numéros de règles passent en infobulle.
        let narrow = ui.available_width() < 520.0;
        egui::Grid::new("ports-table")
            .striped(true)
            .num_columns(if narrow { 3 } else { 5 })
            .min_col_width(10.0)
            .spacing(egui::vec2(16.0, 8.0))
            .show(ui, |ui| {
                ui.label(header(p, t.col_port));
                ui.label(header(p, t.col_action));
                ui.horizontal(|ui| {
                    ui.label(header(p, t.col_dir));
                    // En fenêtre étroite, la direction est la dernière
                    // colonne : c'est elle qui comble la carte.
                    if narrow {
                        components::stretch_cell(ui);
                    }
                });
                if !narrow {
                    ui.label(header(p, "IP"));
                    ui.horizontal(|ui| {
                        ui.label(header(p, t.col_rules));
                        components::stretch_cell(ui);
                    });
                }
                ui.end_row();

                for (to, action, dir, nums, v6) in &specs {
                    ui.label(RichText::new(ellipsize(to, 34)).monospace())
                        .on_hover_text(to);
                    let (col, label) = match action {
                        RuleAction::Allow => (p.success, t.action_allow),
                        RuleAction::Deny => (p.danger, t.action_deny),
                        RuleAction::Reject => (p.warning, t.action_reject),
                        RuleAction::Limit => (p.accent, t.action_limit),
                    };
                    components::action_badge(ui, p, label, col);
                    ui.label(RichText::new(t.rule_dir(*dir)).size(12.5));
                    if narrow {
                        // L'essentiel reste lisible ; le détail complète
                        // l'infobulle du port.
                        ui.end_row();
                        continue;
                    }
                    if *v6 {
                        let bg = crate::components::blend(p.accent, p.card, 0.16);
                        components::badge(ui, p, "IPv6", p.accent, bg);
                    } else {
                        components::badge(ui, p, "IPv4", p.text_secondary, p.card_hover);
                    }
                    ui.label(
                        RichText::new(
                            nums.iter().map(|n| format!("#{n}")).collect::<Vec<_>>().join(", "),
                        )
                        .monospace()
                        .size(11.5)
                        .color(p.text_secondary),
                    );
                    ui.end_row();
                }
            });
    });
}

// ─── Carte aperçu de la commande ─────────────────────────────────────────────
fn preview_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    let empty = app.add_form.port.trim().is_empty() && !app.add_form.use_profile;
    let valid = app.add_form.to_new_rule().is_ok();
    let preview = app.add_form.preview(t);

    components::card(ui, p, t.preview_title, |_| {}, |ui| {
        if valid {
            // Commande ufw sur fond code ; infobulle = texte intégral.
            egui::Frame::NONE
                .fill(p.row_alt)
                .corner_radius(egui::CornerRadius::same(8))
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width() - 2.0);
                    ui.label(
                        RichText::new(ellipsize(&preview, 140))
                            .monospace()
                            .color(p.text),
                    )
                    .on_hover_text(&preview);
                });
        } else if empty {
            // Formulaire vide : pas d'erreur affichée, simple indication.
            components::hint(ui, p, t.hint_port);
        } else {
            // Saisie invalide : message d'erreur localisé.
            ui.label(
                RichText::new(format!("{}  {}", crate::icons::WARNING, preview))
                    .monospace()
                    .color(p.danger),
            );
        }
    });
}

// ─── Carte règles (formulaire inline + liste) ────────────────────────────────
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
            // Sélecteurs segmentés : libellé à gauche si la place suffit,
            // au-dessus du sélecteur sinon (paramètre responsive).
            components::labeled_control(ui, p, t.proto_label, 460.0, |ui| {
                let mut proto = app.add_form.proto;
                components::segmented(
                    ui,
                    p,
                    &[(Proto::Tcp, t.proto_tcp), (Proto::Udp, t.proto_udp), (Proto::Any, t.proto_any)],
                    &mut proto,
                );
                app.add_form.proto = proto;
            });
            components::labeled_control(ui, p, t.dir_label, 460.0, |ui| {
                let mut dir = app.add_form.dir;
                components::segmented(
                    ui,
                    p,
                    &[(NewRuleDir::In, t.dir_inbound), (NewRuleDir::Out, t.dir_outbound)],
                    &mut dir,
                );
                app.add_form.dir = dir;
            });
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

            // Options avancées repliables (dont le commentaire de la règle).
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
            ui.add_space(8.0);

            // ── Recherche + liste des règles ──
            ui.horizontal(|ui| {
                ui.label(crate::icons::icon(crate::icons::SEARCH));
                // Champ de recherche fluide : il occupe l'espace restant,
                // l'ancienneté restant visible à droite.
                let search_w = (ui.available_width() - 170.0).clamp(120.0, 300.0);
                ui.add(
                    egui::TextEdit::singleline(&mut app.firewall.search)
                        .hint_text(t.search_placeholder)
                        .desired_width(search_w),
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

            // Intégralité des règles visible d'un coup d'œil : pas de
            // défilement interne — la carte s'étire à la hauteur du
            // tableau et c'est le défilement de la page qui prend le
            // relais (une ScrollArea imbriquée s'effondrerait à 64 px).
            ui.vertical(|ui| {
                    // Tableau responsive : en fenêtre étroite, les colonnes
                    // « De » et « IP » passent en infobulle (6 ou 4 colonnes ;
                    // le commentaire reste dans l'infobulle de suppression).
                    let narrow = ui.available_width() < 640.0;
                    egui::Grid::new("rules-table")
                        .striped(true)
                        .num_columns(if narrow { 5 } else { 7 })
                        .min_col_width(10.0)
                        .spacing(egui::vec2(14.0, 8.0))
                        .show(ui, |ui| {
                            ui.label(header(p, "#"));
                            ui.label(header(p, t.col_action));
                            if !narrow {
                                ui.label(header(p, t.col_from));
                            }
                            ui.label(header(p, t.col_to_port));
                            ui.label(header(p, t.col_dir));
                            if !narrow {
                                ui.label(header(p, "IP"));
                            }
                            // Dernière colonne (icône de suppression) :
                            // comble la largeur de la carte.
                            ui.horizontal(|ui| {
                                ui.label(header(p, ""));
                                components::stretch_cell(ui);
                            });
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
                                if !narrow {
                                    ui.label(RichText::new(ellipsize(&rule.from, 30)).monospace())
                                        .on_hover_text(&rule.from);
                                }
                                let port_label = RichText::new(ellipsize(&rule.to, 30)).monospace();
                                if narrow {
                                    // La source reste consultable en infobulle.
                                    ui.label(port_label)
                                        .on_hover_text(format!("{}\n← {}", rule.to, rule.from));
                                } else {
                                    ui.label(port_label).on_hover_text(&rule.to);
                                }
                                ui.label(RichText::new(t.rule_dir(rule.dir)).size(12.0).color(p.text_secondary));
                                if !narrow {
                                    // Badge IPv4/IPv6 dédié : lecture immédiate du type d'adresse.
                                    if rule.ipv6 {
                                        let bg = crate::components::blend(p.accent, p.card, 0.16);
                                        components::badge(ui, p, "IPv6", p.accent, bg);
                                    } else {
                                        components::badge(ui, p, "IPv4", p.text_secondary, p.card_hover);
                                    }
                                }
                                let mut desc = format!(
                                    "#{} — {} {} ({})",
                                    rule.number,
                                    t.action(rule.action),
                                    rule.to,
                                    rule.from
                                );
                                if let Some(c) = &rule.comment {
                                    desc.push_str(&format!("\n{c}"));
                                }
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
