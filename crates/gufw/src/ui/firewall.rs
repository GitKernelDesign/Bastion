//! Écran Pare-feu : état, politiques par défaut, journalisation, règles.

use crate::app::{ConnPhase, ConfirmData, ConfirmKind, GuiApp, Modal, Pending};
use crate::components;
use crate::theme::Palette;
use egui::{Align, Layout, RichText};
use gufw_core::model::*;
use gufw_core::op::Op;

pub fn render(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    components::screen_header(
        ui,
        p,
        "Pare-feu",
        "État du pare-feu UFW, politiques par défaut et règles en vigueur.",
    );
    ui.add_space(6.0);

    if app.phase != ConnPhase::Connected {
        components::empty_state(
            ui,
            p,
            "Non connecté",
            "Connectez-vous d'abord depuis l'écran Connexion (Ctrl+1).",
        );
        return;
    }

    // ---- rangée de cartes d'état
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(12.0, 12.0);
        status_card(app, ui, p);
        policy_card(app, ui, p);
        logging_card(app, ui, p);
    });

    ui.add_space(12.0);

    // ---- règles
    rules_card(app, ui, p);

    ui.add_space(12.0);

    // ---- actions rares (en bas, vers le regard expert)
    ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
        let busy = app.pending.values().any(|x| matches!(x, Pending::Report));
        if ui
            .add_enabled(!busy, egui::Button::new(components::btn_icon_text(crate::icons::TERMINAL, "Rapport UFW")))
            .clicked()
        {
            app.send_op(Pending::Report, Op::UfwReport);
        }
        if components::danger_button(ui, p, "Réinitialiser…").clicked() {
            app.modal = Some(Modal::ResetFw {
                typed: String::new(),
            });
        }
        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
            let busy = app.pending.values().any(|x| matches!(x, Pending::Reload));
            if ui
                .add_enabled(!busy, egui::Button::new(components::btn_icon_text(crate::icons::RESTART, "Recharger")))
                .on_hover_text("Recharge les règles (ufw reload)")
                .clicked()
            {
                app.send_op(Pending::Reload, Op::UfwReload);
            }
        });
    });
}

/// Carte d'état : interrupteur principal + version.
fn status_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    components::card_body(ui, p, |ui| {
        ui.set_min_width(230.0);
        let (active, version, raw) = match &app.firewall.status {
            Some(s) => (s.active, probe_ufw_version(app), s.raw.clone()),
            None => (false, String::new(), String::new()),
        };
        let busy = app
            .pending
            .values()
            .any(|x| matches!(x, Pending::Enable | Pending::Disable | Pending::RefreshStatus));

        components::status_line(
            ui,
            p,
            if active { p.success } else { p.danger },
            if active { "Actif" } else { "Désactivé" },
            None,
        );
        if !version.is_empty() {
            components::hint(ui, p, &version);
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let on = components::switch(ui, p, active, if active { Some(p.success) } else { None });
            if on != active && !busy {
                if on {
                    app.send_op(Pending::Enable, Op::UfwEnable);
                } else {
                    // Désactiver est une action destructrice : confirmation.
                    app.modal = Some(Modal::Confirm(ConfirmData {
                        title: "Désactiver le pare-feu ?".into(),
                        body: "Toutes les protections de la machine cible seront suspendues : les connexions entrantes ne seront plus filtrées. Cette action prend effet immédiatement.".into(),
                        confirm_label: "Désactiver".into(),
                        kind: ConfirmKind::DisableFw,
                    }));
                }
            }
            if busy {
                ui.add(egui::Spinner::new().size(14.0));
            }
        });
        let _ = raw;
        ui.add_space(2.0);
        if ui
            .add(egui::Button::new(RichText::new("Sortie brute").size(11.5)))
            .clicked()
        {
            app.firewall.show_raw = !app.firewall.show_raw;
        }
        if app.firewall.show_raw {
            if let Some(s) = &app.firewall.status {
                egui::ScrollArea::vertical()
                    .id_salt("ufw-raw")
                    .max_height(140.0)
                    .show(ui, |ui| {
                        ui.monospace(&s.raw);
                    });
            }
        }
    });
}

fn probe_ufw_version(app: &GuiApp) -> String {
    app.probe.0.clone().unwrap_or_default()
}

/// Carte politiques par défaut.
fn policy_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    components::card_body(ui, p, |ui| {
        ui.set_min_width(250.0);
        ui.label(
            RichText::new("POLITIQUES PAR DÉFAUT")
                .size(11.5)
                .weak()
                .family(egui::FontFamily::Name("Medium".into())),
        );
        ui.add_space(6.0);
        let st = app.firewall.status.clone();
        let Some(st) = st else {
            components::hint(ui, p, "État inconnu (actualisez).");
            return;
        };

        for dir in [Direction::Incoming, Direction::Outgoing, Direction::Routed] {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(match dir {
                        Direction::Incoming => "Entrée",
                        Direction::Outgoing => "Sortie",
                        Direction::Routed => "Routage",
                    })
                    .size(12.5)
                    .color(p.text_secondary),
                );
                let current = match dir {
                    Direction::Incoming => st.defaults.incoming.unwrap_or(Policy::Deny),
                    Direction::Outgoing => st.defaults.outgoing.unwrap_or(Policy::Allow),
                    Direction::Routed => {
                        if st.defaults.routed_disabled {
                            Policy::Deny
                        } else {
                            st.defaults.routed.unwrap_or(Policy::Deny)
                        }
                    }
                };
                let mut sel = current;
                egui::ComboBox::from_id_salt(format!("policy-{:?}", dir))
                    .selected_text(current.libelle())
                    .width(110.0)
                    .show_ui(ui, |ui| {
                        for pol in [Policy::Allow, Policy::Deny, Policy::Reject] {
                            ui.selectable_value(&mut sel, pol, pol.libelle());
                        }
                    });
                if dir == Direction::Routed && st.defaults.routed_disabled {
                    ui.label(
                        RichText::new("(relais noyau désactivé)")
                            .size(10.5)
                            .color(p.warning),
                    );
                }
                if sel != current {
                    app.send_op(
                        Pending::Policy(dir),
                        Op::UfwSetDefault { dir, policy: sel },
                    );
                }
            });
        }
        ui.add_space(2.0);
        components::hint(
            ui,
            p,
            "Rejeter = refuse avec notification à l'émetteur ; Refuser = silencieux.",
        );
    });
}

/// Carte journalisation.
fn logging_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    components::card_body(ui, p, |ui| {
        ui.set_min_width(230.0);
        ui.label(
            RichText::new("JOURNALISATION")
                .size(11.5)
                .weak()
                .family(egui::FontFamily::Name("Medium".into())),
        );
        ui.add_space(6.0);
        let Some(st) = app.firewall.status.clone() else {
            components::hint(ui, p, "État inconnu.");
            return;
        };
        let current = if st.logging_on {
            st.logging_level.unwrap_or(LogLevel::Low)
        } else {
            LogLevel::Off
        };
        let mut sel = current;
        egui::ComboBox::from_id_salt("log-level")
            .selected_text(current.libelle())
            .width(150.0)
            .show_ui(ui, |ui| {
                for lvl in [
                    LogLevel::Off,
                    LogLevel::Low,
                    LogLevel::Medium,
                    LogLevel::High,
                    LogLevel::Full,
                ] {
                    ui.selectable_value(&mut sel, lvl, lvl.libelle());
                }
            });
        if sel != current {
            app.send_op(Pending::Logging, Op::UfwSetLogging { level: sel });
        }
        ui.add_space(4.0);
        components::hint(
            ui,
            p,
            "Les blocages sont consultables dans l'écran Fail2Ban (« Journaux »).",
        );
    });
}

/// Carte règles : recherche, table, ajout, suppression.
fn rules_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let n = app.firewall.rules.len();
    let mut refresh_clicked = false;
    let mut add_clicked = false;
    components::card(
        ui,
        p,
        &format!("Règles ({n})"),
        |ui| {
            if components::icon_button(ui, p, crate::icons::REFRESH, "Actualiser les règles")
                .clicked()
            {
                refresh_clicked = true;
            }
            if components::primary_button_small(ui, p, crate::icons::ADD, "Ajouter").clicked() {
                add_clicked = true;
            }
        },
        |ui| {
            // Recherche
            ui.horizontal(|ui| {
                ui.label(crate::icons::icon(crate::icons::SEARCH).size(14.0));
                ui.add(
                    egui::TextEdit::singleline(&mut app.firewall.search)
                        .hint_text("Filtrer (port, adresse, commentaire…)")
                        .desired_width(280.0),
                );
                if let Some(at) = app.firewall.rules_at {
                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                        components::since_label(ui, p, at);
                    });
                }
            });
            ui.add_space(8.0);

            if n == 0 {
                components::empty_state(
                    ui,
                    p,
                    "Aucune règle",
                    "Cliquez sur « Ajouter » pour autoriser un port ou une application.",
                );
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

            let n_cols = 7;
            egui::ScrollArea::vertical()
                .max_height(320.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    egui::Grid::new("rules-table")
                        .striped(true)
                        .num_columns(n_cols)
                        .min_col_width(10.0)
                        .spacing(egui::vec2(14.0, 8.0))
                        .show(ui, |ui| {
                            ui.label(header_txt(p, "#"));
                            ui.label(header_txt(p, "Action"));
                            ui.label(header_txt(p, "De"));
                            ui.label(header_txt(p, "Vers / Port"));
                            ui.label(header_txt(p, "Sens"));
                            ui.label(header_txt(p, "Commentaire"));
                            ui.label(header_txt(p, ""));
                            ui.end_row();

                            for rule in &rules {
                                ui.label(RichText::new(rule.number.to_string()).weak());
                                let (color, label) = match rule.action {
                                    RuleAction::Allow => (p.success, "Autoriser"),
                                    RuleAction::Deny => (p.danger, "Bloquer"),
                                    RuleAction::Reject => (p.warning, "Rejeter"),
                                    RuleAction::Limit => (p.accent, "Limiter"),
                                };
                                components::action_badge(ui, p, label, color);
                                ui.label(RichText::new(&rule.from).monospace());
                                ui.label(RichText::new(&rule.to).monospace());
                                ui.label(
                                    RichText::new(direction_txt(rule))
                                        .size(12.0)
                                        .color(p.text_secondary),
                                );
                                match &rule.comment {
                                    Some(c) => ui.label(RichText::new(c).size(12.5)),
                                    None => ui.label(RichText::new("—").weak()),
                                };
                                let desc = format!(
                                    "#{} — {} {} (de {})",
                                    rule.number,
                                    rule.action.libelle(),
                                    rule.to,
                                    rule.from
                                );
                                if components::icon_button(
                                    ui,
                                    p,
                                    crate::icons::DELETE,
                                    &format!("Supprimer la règle {desc}"),
                                )
                                .clicked()
                                {
                                    app.modal = Some(Modal::Confirm(ConfirmData {
                                        title: "Supprimer la règle ?".into(),
                                        body: desc,
                                        confirm_label: "Supprimer".into(),
                                        kind: ConfirmKind::DeleteRule { index: rule.number },
                                    }));
                                }
                                ui.end_row();
                            }
                        });
                });
            ui.add_space(2.0);
            components::hint(
                ui,
                p,
                "Les adresses et ports sont en police mono : comparables d'une ligne à l'autre. "
                    .to_string()
                    .as_str(),
            );
            ui.label(
                RichText::new("Le badge « v6 » signale la déclinaison IPv6 d'une règle.")
                    .size(12.5)
                    .color(p.text_secondary),
            );
        },
    );

    if refresh_clicked {
        app.send_op(Pending::RefreshRules, Op::UfwRules);
    }
    if add_clicked {
        app.add_form = crate::app::AddRuleForm::default();
        if app.firewall.profiles.is_empty() {
            app.send_op(Pending::RefreshProfiles, Op::UfwAppProfiles);
        }
        app.modal = Some(Modal::AddRule);
    }
}

fn direction_txt(rule: &Rule) -> String {
    let mut s = rule.dir.libelle().to_string();
    if rule.ipv6 {
        s.push_str(" · v6");
    }
    s
}

fn header_txt(p: &Palette, s: &str) -> RichText {
    RichText::new(s)
        .size(11.5)
        .weak()
        .family(egui::FontFamily::Name("Medium".into()))
        .color(p.text_secondary)
}
