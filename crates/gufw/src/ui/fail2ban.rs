//! Écran Fail2Ban : service, prisons, IP bannies, bannissement manuel.

use crate::app::{ConnPhase, ConfirmData, ConfirmKind, GuiApp, Modal, Pending};
use crate::components;
use crate::theme::Palette;
use egui::{Align, Layout, RichText};
use gufw_core::fail2ban::humanize_duration;
use gufw_core::model::{JailInfo, LogSource};
use gufw_core::op::{Op, ServiceAction};

pub fn render(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    components::screen_header(
        ui,
        p,
        "Fail2Ban",
        "Service, prisons actives et adresses bannies, avec actions de bannissement.",
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

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(12.0, 12.0);
        service_card(app, ui, p);
        summary_card(app, ui, p);
    });

    ui.add_space(12.0);

    jails_card(app, ui, p);

    if let Some(jail) = app.f2b.selected_jail.clone() {
        ui.add_space(12.0);
        jail_card(app, ui, p, &jail);
    }
}

fn service_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    components::card_body(ui, p, |ui| {
        ui.set_min_width(300.0);
        let state = app.f2b.service.clone().unwrap_or_else(|| "inconnu".into());
        let active = state == "active";
        let color = if active {
            p.success
        } else if state == "failed" {
            p.danger
        } else {
            p.text_secondary
        };
        components::status_line(ui, p, color, "Service", Some(&state));
        if let Some(at) = app.f2b.service_at {
            components::since_label(ui, p, at);
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if active {
                if ui
                    .add(egui::Button::new(components::btn_icon_text(crate::icons::RESTART, "Redémarrer")))
                    .clicked()
                {
                    app.modal = Some(Modal::Confirm(ConfirmData {
                        title: "Redémarrer Fail2Ban ?".into(),
                        body: "Les prisons seront réinitialisées ; les bannissements en cours seront perdus.".into(),
                        confirm_label: "Redémarrer".into(),
                        kind: ConfirmKind::RestartF2b,
                    }));
                }
                if ui
                    .add(egui::Button::new(components::btn_icon_text(crate::icons::STOP, "Arrêter")))
                    .clicked()
                {
                    app.modal = Some(Modal::Confirm(ConfirmData {
                        title: "Arrêter Fail2Ban ?".into(),
                        body: "Le service ne surveillera plus les journaux et ne bannira plus personne.".into(),
                        confirm_label: "Arrêter".into(),
                        kind: ConfirmKind::StopF2b,
                    }));
                }
            } else {
                if components::primary_button(ui, p, "Démarrer").clicked() {
                    app.send_op(
                        Pending::Service(ServiceAction::Start),
                        Op::F2bServiceAction { action: ServiceAction::Start },
                    );
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                if ui
                    .add(egui::Button::new(components::btn_icon_text(crate::icons::REFRESH, "Recharger")))
                    .on_hover_text("fail2ban-client reload (relit la configuration)")
                    .clicked()
                {
                    app.send_op(Pending::ReloadJail, Op::F2bReloadJail { jail: None });
                }
            });
        });
    });
}

fn summary_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    components::card_body(ui, p, |ui| {
        ui.set_min_width(220.0);
        let (jails, banned) = match &app.f2b.global {
            Some(g) => {
                let b: u64 = g.jails
                    .iter()
                    .filter_map(|j| app.f2b.jail_infos.get(j).map(|(info, _)| info.currently_banned.unwrap_or(0)))
                    .sum();
                (g.jails.len(), b)
            }
            None => (0, 0),
        };
        ui.horizontal(|ui| {
            big_number(ui, p, jails.to_string(), "prisons");
            ui.add_space(16.0);
            big_number(ui, p, banned.to_string(), "bannies (prisons ouvertes)");
        });
        if let Some(at) = app.f2b.global_at {
            ui.add_space(4.0);
            components::since_label(ui, p, at);
        }
    });
}

fn big_number(ui: &mut egui::Ui, p: &Palette, n: String, label: &str) {
    ui.vertical(|ui| {
        ui.label(
            RichText::new(n)
                .size(26.0)
                .color(p.text)
                .family(egui::FontFamily::Name("SemiBold".into())),
        );
        ui.label(RichText::new(label).size(11.5).color(p.text_secondary));
    });
}

fn jails_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let jails: Vec<String> = app
        .f2b
        .global
        .as_ref()
        .map(|g| g.jails.clone())
        .unwrap_or_default();
    let mut refresh_all_clicked = false;
    let mut logs_clicked = false;
    let mut banned_all_clicked = false;
    components::card(
        ui,
        p,
        &format!("Prisons ({})", jails.len()),
        |ui| {
            if components::icon_button(ui, p, crate::icons::REFRESH, "Actualiser").clicked() {
                refresh_all_clicked = true;
            }
            if ui
                .add(egui::Button::new(components::btn_icon_text(crate::icons::BAN, "Toutes les IP")))
                .on_hover_text("Liste globale des IP bannies (fail2ban-client banned)")
                .clicked()
            {
                banned_all_clicked = true;
            }
            if ui
                .add(egui::Button::new(components::btn_icon_text(crate::icons::TERMINAL, "Journal")))
                .on_hover_text("journalctl -u fail2ban")
                .clicked()
            {
                logs_clicked = true;
            }
        },
        |ui| {
            if jails.is_empty() {
                components::empty_state(
                    ui,
                    p,
                    "Aucune prison active",
                    "Fail2Ban n'a signalé aucune prison (ou le service est arrêté).",
                );
                return;
            }
            egui::ScrollArea::vertical()
                .max_height(220.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    egui::Grid::new("jails-table")
                        .striped(true)
                        .num_columns(6)
                        .spacing(egui::vec2(16.0, 8.0))
                        .show(ui, |ui| {
                            ui.label(header_txt(p, "Prison"));
                            ui.label(header_txt(p, "Bannies"));
                            ui.label(header_txt(p, "Échecs"));
                            ui.label(header_txt(p, "bantime"));
                            ui.label(header_txt(p, "findtime"));
                            ui.label(header_txt(p, ""));
                            ui.end_row();

                            for jail in &jails {
                                let selected = app.f2b.selected_jail.as_deref() == Some(jail.as_str());
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new(jail.clone())
                                                .monospace()
                                                .color(if selected { p.accent } else { p.text }),
                                        )
                                        .fill(egui::Color32::TRANSPARENT),
                                    )
                                    .clicked()
                                {
                                    app.f2b.selected_jail = Some(jail.clone());
                                    app.send_op(
                                        Pending::RefreshJail,
                                        Op::F2bJail { jail: jail.clone() },
                                    );
                                }
                                if let Some((info, _)) = app.f2b.jail_infos.get(jail) {
                                    ui.label(RichText::new(
                                        info.currently_banned.map(|b| b.to_string()).unwrap_or("—".into()),
                                    ));
                                    ui.label(RichText::new(
                                        info.currently_failed.map(|f| f.to_string()).unwrap_or("—".into()),
                                    ));
                                    ui.label(
                                        RichText::new(
                                            info.bantime.as_deref().map(humanize_duration).unwrap_or("—".into()),
                                        )
                                        .size(12.5),
                                    );
                                    ui.label(
                                        RichText::new(
                                            info.findtime.as_deref().map(humanize_duration).unwrap_or("—".into()),
                                        )
                                        .size(12.5),
                                    );
                                } else {
                                    ui.label(RichText::new("…").weak());
                                    ui.label(RichText::new("").weak());
                                    ui.label(RichText::new("…").weak());
                                    ui.label(RichText::new("").weak());
                                }
                                ui.label(RichText::new(""));
                                ui.end_row();
                            }
                        });
                });
        },
    );

    if refresh_all_clicked {
        for jail in &jails {
            app.send_op(
                Pending::RefreshJail,
                Op::F2bJail { jail: jail.clone() },
            );
        }
    }
    if banned_all_clicked {
        app.send_op(Pending::BannedAll, Op::F2bBannedAll);
    }
    if logs_clicked {
        app.send_op(
            Pending::Logs(LogSource::Fail2ban),
            Op::F2bLogs {
                which: LogSource::Fail2ban,
                lines: 300,
            },
        );
    }
}

fn jail_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette, jail: &str) {
    let (info, at) = match app.f2b.jail_infos.get(jail) {
        Some((i, t)) => (i.clone(), *t),
        None => (JailInfo::default(), std::time::Instant::now()),
    };

    let mut refresh_clicked = false;
    let mut reload_clicked = false;
    components::card(
        ui,
        p,
        &format!("Prison « {jail} »"),
        |ui| {
            if components::icon_button(ui, p, crate::icons::REFRESH, "Actualiser la prison")
                .clicked()
            {
                refresh_clicked = true;
            }
            if ui
                .add(egui::Button::new(components::btn_icon_text(crate::icons::RESTART, "Recharger")))
                .on_hover_text("Relit la configuration de cette prison")
                .clicked()
            {
                reload_clicked = true;
            }
        },
        |ui| {
            ui.horizontal(|ui| {
                components::hint(ui, p, "Échecs actuels :");
                ui.label(opt_u64(info.currently_failed));
                components::hint(ui, p, "· Total :");
                ui.label(opt_u64(info.total_failed));
                components::hint(ui, p, "· maxretry :");
                ui.label(
                    info.maxretry.clone().unwrap_or_else(|| "—".into()),
                );
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    components::since_label(ui, p, at);
                });
            });
            if let Some(files) = &info.file_list {
                ui.horizontal(|ui| {
                    components::hint(ui, p, "Surveille :");
                    ui.monospace(files);
                });
            }
            ui.add_space(8.0);

            // Bannissement manuel
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut app.f2b.ban_input)
                        .hint_text("Adresse IP à bannir")
                        .desired_width(220.0)
                        .font(egui::TextStyle::Monospace),
                );
                let ok = gufw_core::ufw::is_valid_addr(app.f2b.ban_input.trim());
                if ui
                    .add_enabled(ok, egui::Button::new(components::btn_icon_text(crate::icons::BAN, "Bannir")))
                    .clicked()
                {
                    let ip = app.f2b.ban_input.trim().to_string();
                    app.f2b.ban_input.clear();
                    app.send_op(
                        Pending::Ban {
                            jail: jail.to_string(),
                            ip: ip.clone(),
                        },
                        Op::F2bBanIp {
                            jail: jail.to_string(),
                            ip,
                        },
                    );
                }
            });

            ui.add_space(8.0);

            // Liste des bannies
            ui.label(
                RichText::new(format!("IP BANNIES ({})", info.banned_ips.len()))
                    .size(11.5)
                    .weak()
                    .family(egui::FontFamily::Name("Medium".into())),
            );
            ui.add_space(4.0);
            if info.banned_ips.is_empty() {
                components::hint(ui, p, "Aucune adresse bannie dans cette prison.");
                return;
            }
            egui::ScrollArea::vertical()
                .max_height(240.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    egui::Grid::new("banned-table")
                        .striped(true)
                        .num_columns(2)
                        .spacing(egui::vec2(16.0, 6.0))
                        .show(ui, |ui| {
                            for ip in &info.banned_ips {
                                ui.label(RichText::new(ip).monospace());
                                if ui
                                    .add(egui::Button::new(components::btn_icon_text(
                                        crate::icons::RESTORE,
                                        "Débannir",
                                    )))
                                    .clicked()
                                {
                                    app.send_op(
                                        Pending::Unban {
                                            jail: jail.to_string(),
                                            ip: ip.clone(),
                                        },
                                        Op::F2bUnbanIp {
                                            jail: jail.to_string(),
                                            ip: ip.clone(),
                                        },
                                    );
                                }
                                ui.end_row();
                            }
                        });
                });
        },
    );

    if refresh_clicked {
        app.send_op(
            Pending::RefreshJail,
            Op::F2bJail { jail: jail.to_string() },
        );
    }
    if reload_clicked {
        app.send_op(
            Pending::ReloadJail,
            Op::F2bReloadJail { jail: Some(jail.to_string()) },
        );
    }
}

fn opt_u64(v: Option<u64>) -> RichText {
    RichText::new(v.map(|x| x.to_string()).unwrap_or_else(|| "—".into()))
        .monospace()
}

fn header_txt(p: &Palette, s: &str) -> RichText {
    RichText::new(s)
        .size(11.5)
        .weak()
        .family(egui::FontFamily::Name("Medium".into()))
        .color(p.text_secondary)
}
