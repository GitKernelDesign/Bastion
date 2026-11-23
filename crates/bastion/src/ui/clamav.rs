//! Écran Antivirus (ClamAV) : services, bases de signatures, analyse à
//! la demande et résultat en place. Lecture seule pour les fichiers : aucune
//! suppression ni quarantaine automatique.

use crate::app::{ConfirmData, ConfirmKind, GuiApp, Modal, Pending};
use crate::components;
use crate::icons;
use crate::theme::{space, text, Palette};
use bastion_core::model::ClamLogSource;
use bastion_core::op::{Op, ServiceAction};
use egui::{Align, CornerRadius, Frame, Layout, Margin, RichText};

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

    // ClamAV installé ? (unités introuvables côté systemd)
    let daemon_state = app.clam.daemon.clone().unwrap_or_default();
    let freshclam_state = app.clam.freshclam.clone().unwrap_or_default();
    let not_installed = app.clam.daemon.is_some()
        && app.clam.freshclam.is_some()
        && daemon_state == "not-found"
        && freshclam_state == "not-found";

    if not_installed {
        components::card_body(ui, p, |ui| {
            components::empty_state(ui, p, t.nav_clamav, t.clam_not_installed);
        });
        return;
    }

    components::tile_grid(ui, p, 360.0, 2, |ui, p, i| match i {
        0 => services_card(app, ui, p),
        _ => db_card(app, ui, p),
    });
    ui.add_space(space::M);
    scan_card(app, ui, p);
}

// ─── Services ClamAV ─────────────────────────────────────────────────────────
fn services_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    let mut refresh = false;
    let mut action: Option<(&'static str, ServiceAction)> = None;

    let daemon_state = app
        .clam
        .daemon
        .clone()
        .unwrap_or_else(|| t.state_unknown.to_string());
    let freshclam_state = app
        .clam
        .freshclam
        .clone()
        .unwrap_or_else(|| t.state_unknown.to_string());

    components::card(
        ui,
        p,
        t.clam_services_title,
        |ui| {
            if components::icon_button(ui, p, icons::REFRESH, t.tooltip_refresh).clicked() {
                refresh = true;
            }
        },
        |ui| {
            service_row(
                app,
                ui,
                p,
                "clamav-daemon",
                &daemon_state,
                app.clam.daemon_at,
                &mut action,
            );
            ui.add_space(space::M);
            components::hairline(ui, p);
            ui.add_space(space::M);
            service_row(
                app,
                ui,
                p,
                "clamav-freshclam",
                &freshclam_state,
                app.clam.freshclam_at,
                &mut action,
            );
        },
    );

    if refresh {
        app.send_op(Pending::ClamServices, Op::ClamServicesState);
    }
    if let Some((unit, act)) = action {
        match act {
            ServiceAction::Stop => {
                app.modal = Some(Modal::Confirm(ConfirmData {
                    title: t.clam_stop_title.to_string(),
                    body: t.clam_stop_body.to_string(),
                    confirm_label: t.btn_stop.to_string(),
                    kind: ConfirmKind::StopClamav {
                        unit: unit.to_string(),
                    },
                }));
            }
            other => {
                app.send_op(
                    Pending::ClamService {
                        unit: unit.to_string(),
                        action: other,
                    },
                    Op::ClamServiceAction {
                        unit: unit.to_string(),
                        action: other,
                    },
                );
            }
        }
    }
}

fn service_row(
    app: &GuiApp,
    ui: &mut egui::Ui,
    p: &Palette,
    name: &'static str,
    state: &str,
    at: Option<std::time::Instant>,
    action: &mut Option<(&'static str, ServiceAction)>,
) {
    let t = app.tr;
    let busy = app
        .pending
        .values()
        .any(|x| matches!(x, Pending::ClamService { unit, .. } if unit == name));
    let active = state == "active";
    let col = if active {
        p.success
    } else if state == "failed" {
        p.danger
    } else {
        p.text_secondary
    };

    ui.vertical(|ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 6.0);
            components::status_dot(ui, col, 8.0);
            ui.label(
                RichText::new(name)
                    .monospace()
                    .family(egui::FontFamily::Name("MonoMedium".into())),
            );
            components::tag(ui, p, state, col);
            if let Some(at) = at {
                components::since_label(ui, p, t, at);
            }
            if busy {
                ui.add(egui::Spinner::new().size(12.0));
            }
        });
        ui.add_space(space::XS);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
            if active {
                if components::ghost_button(ui, p, t.btn_restart).clicked() {
                    *action = Some((name, ServiceAction::Restart));
                }
                if components::styled_button(ui, p, t.btn_stop, components::BtnKind::Ghost)
                    .clicked()
                {
                    *action = Some((name, ServiceAction::Stop));
                }
            } else if components::primary_button(ui, p, t.btn_start).clicked() {
                *action = Some((name, ServiceAction::Start));
            }
        });
    });
}

// ─── Bases de signatures ─────────────────────────────────────────────────────
fn db_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    let mut refresh = false;
    let mut update = false;
    let mut logs_daemon = false;
    let mut logs_freshclam = false;
    let db = app.clam.db.clone();
    let db_at = app.clam.db_at;

    components::card(
        ui,
        p,
        t.clam_db_title,
        |ui| {
            if let Some(at) = db_at {
                components::since_label(ui, p, t, at);
                ui.add_space(2.0);
            }
            if components::icon_button(ui, p, icons::REFRESH, t.tooltip_refresh).clicked() {
                refresh = true;
            }
            if components::styled_button(
                ui,
                p,
                &format!("{}  {}", icons::UPDATE, t.clam_update_btn),
                components::BtnKind::Primary,
            )
            .clicked()
            {
                update = true;
            }
        },
        |ui| {
            match &db {
                Some(db) => {
                    egui::Grid::new("clam-db")
                        .num_columns(2)
                        .spacing(egui::vec2(14.0, 8.0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(t.clam_db_engine)
                                    .size(text::SMALL)
                                    .color(p.text_secondary),
                            );
                            ui.label(RichText::new(&db.engine).monospace());
                            ui.end_row();
                            ui.label(
                                RichText::new(t.clam_db_sigs)
                                    .size(text::SMALL)
                                    .color(p.text_secondary),
                            );
                            ui.label(
                                RichText::new(thousands(db.sigs))
                                    .monospace()
                                    .family(egui::FontFamily::Name("MonoMedium".into())),
                            );
                            ui.end_row();
                            ui.label(
                                RichText::new(t.clam_db_date)
                                    .size(text::SMALL)
                                    .color(p.text_secondary),
                            );
                            ui.label(RichText::new(&db.db_date).monospace());
                            ui.end_row();
                        });
                }
                None => {
                    components::hint(ui, p, t.clam_db_unknown);
                }
            }
            ui.add_space(space::M);
            components::hairline(ui, p);
            ui.add_space(space::M);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 6.0);
                if components::styled_button(
                    ui,
                    p,
                    &format!("{}  {}", icons::ARTICLE, t.clam_logs_daemon_btn),
                    components::BtnKind::Quiet,
                )
                .clicked()
                {
                    logs_daemon = true;
                }
                if components::styled_button(
                    ui,
                    p,
                    &format!("{}  {}", icons::ARTICLE, t.clam_logs_freshclam_btn),
                    components::BtnKind::Quiet,
                )
                .clicked()
                {
                    logs_freshclam = true;
                }
            });
        },
    );

    if refresh {
        app.send_op(Pending::ClamDb, Op::ClamDbInfo);
    }
    if update {
        app.send_op(Pending::ClamUpdate, Op::ClamUpdate);
    }
    if logs_daemon {
        app.send_op(
            Pending::ClamLogs,
            Op::ClamLogs {
                which: ClamLogSource::Daemon,
                lines: 300,
            },
        );
    }
    if logs_freshclam {
        app.send_op(
            Pending::ClamLogs,
            Op::ClamLogs {
                which: ClamLogSource::Freshclam,
                lines: 300,
            },
        );
    }
}

/// 2861247 → « 2 861 247 » (fine insécable, lisible en tableau).
fn thousands(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push('\u{202f}');
        }
        out.push(c);
    }
    out
}

// ─── Analyse à la demande ────────────────────────────────────────────────────
fn scan_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    let mut scan = false;
    let mut copy = false;
    let busy = app.pending.values().any(|x| matches!(x, Pending::ClamScan));
    let daemon_active = app.clam.daemon.as_deref() == Some("active");
    // Le sélecteur de dossier n'a de sens qu'en local : en SSH, la cible
    // est distante et seul le chemin saisi à la main s'applique.
    let local_target = !app.conn_desc.starts_with("SSH");

    components::card(ui, p, t.clam_scan_title, |_| {}, |ui| {
        ui.label(
            RichText::new(t.clam_scan_path_label)
                .size(text::SMALL)
                .color(p.text_secondary)
                .family(egui::FontFamily::Name("Medium".into())),
        );
        let avail = ui.available_width();
        let inline = avail >= 560.0;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            let resp = ui.add(
                egui::TextEdit::singleline(&mut app.clam.scan_path)
                    .hint_text(t.clam_scan_hint)
                    .desired_width(if inline {
                        (avail - 330.0).max(160.0)
                    } else {
                        (ui.available_width() - 8.0).max(160.0)
                    })
                    .margin(Margin::symmetric(10, 8))
                    .font(egui::TextStyle::Monospace),
            );
            let path_ok = {
                let s = app.clam.scan_path.trim();
                s.starts_with('/')
                    && !s.starts_with("/proc")
                    && !s.starts_with("/sys")
                    && !s.starts_with("/dev")
                    && !s.starts_with("/run")
            };
            let enabled = path_ok && !busy && !app.clam.picking;
            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && enabled {
                scan = true;
            }
            if components::styled_button_en(
                ui,
                p,
                &format!("{}  {}", icons::FOLDER, t.clam_browse_btn),
                components::BtnKind::Soft,
                local_target && !app.clam.picking,
            )
            .clicked()
            {
                app.clam.picking = true;
                app.worker.send(crate::worker::Job::PickFolder);
            }
            let label = if busy {
                format!("    {}", t.clam_scan_btn)
            } else {
                format!("{}  {}", icons::SEARCH, t.clam_scan_btn)
            };
            let busy_btn = components::styled_button_minw(
                ui,
                p,
                &label,
                components::BtnKind::Primary,
                enabled,
                150.0,
            );
            if busy {
                let r = busy_btn.rect;
                egui::Spinner::new().size(15.0).paint_at(
                    ui,
                    egui::Rect::from_min_size(
                        egui::pos2(r.left() + 14.0, r.center().y - 7.5),
                        egui::vec2(15.0, 15.0),
                    ),
                );
            }
            if busy_btn.clicked() {
                scan = true;
            }
        });
        ui.add_space(space::S);
        components::hint(ui, p, t.clam_scan_daemon_note);

        // Résultat de la dernière analyse, en place (pas de popin).
        ui.add_space(space::M);
        match &app.clam.last_scan {
            None => {
                components::hint(ui, p, t.clam_scan_none_yet);
            }
            Some((infected, at)) => {
                let clean = infected.is_empty();
                let tone = if clean { p.success } else { p.danger };
                Frame::NONE
                    .fill(p.soft(tone))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(Margin::same(12))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(icons::icon_sz(
                                if clean {
                                    icons::CHECK_CIRCLE
                                } else {
                                    icons::WARNING
                                },
                                16.0,
                                tone,
                            ));
                            ui.label(
                                RichText::new(t.clam_last_scan)
                                    .size(text::SMALL)
                                    .color(p.text)
                                    .family(egui::FontFamily::Name("Medium".into())),
                            );
                            let verdict = if clean {
                                t.clam_scan_clean_short.to_string()
                            } else {
                                format!("{} {}", infected.len(), t.clam_scan_threats)
                            };
                            ui.label(RichText::new(verdict).size(text::SMALL).color(tone));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                components::since_label(ui, p, t, *at);
                            });
                        });
                        if !clean {
                            ui.add_space(space::S);
                            for (path, virus) in infected.iter().take(12) {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(RichText::new(path).monospace().size(text::MONO));
                                    ui.label(
                                        RichText::new(virus)
                                            .monospace()
                                            .size(text::MICRO)
                                            .color(p.danger),
                                    );
                                });
                            }
                            if infected.len() > 12 {
                                components::hint(ui, p, "…");
                            }
                            ui.add_space(space::S);
                            if components::styled_button(
                                ui,
                                p,
                                &format!("{}  {}", icons::COPY, t.copy_btn),
                                components::BtnKind::Ghost,
                            )
                            .clicked()
                            {
                                copy = true;
                            }
                        }
                    });
            }
        }
    });

    if scan && !busy {
        let path = app.clam.scan_path.trim().to_string();
        app.send_op(
            Pending::ClamScan,
            Op::ClamScan {
                path,
                use_daemon: daemon_active,
            },
        );
    }
    if copy {
        if let Some((infected, _)) = &app.clam.last_scan {
            let text = infected
                .iter()
                .map(|(p, v)| format!("{p}\t{v}"))
                .collect::<Vec<_>>()
                .join("\n");
            ui.ctx().copy_text(text);
            app.toasts.info(t.journal_copied_toast.to_string());
        }
    }
}
