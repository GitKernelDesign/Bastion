//! Écran Antivirus (ClamAV) : services, bases de signatures, analyse à
//! la demande, journaux. Lecture seule pour les fichiers : aucune
//! suppression ni quarantaine automatique.

use crate::app::{ConfirmData, ConfirmKind, GuiApp, Modal, Pending};
use crate::components;
use crate::theme::Palette;
use bastion_core::model::ClamLogSource;
use bastion_core::op::{Op, ServiceAction};
use egui::RichText;

pub fn render(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    components::screen_header(ui, p, t.nav_clamav, t.clam_subtitle);
    ui.add_space(6.0);

    if app.phase != crate::app::ConnPhase::Connected {
        components::card_body(ui, p, |ui| {
            components::empty_state(ui, p, t.status_disconnected, t.need_connection_hint);
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

    // Grille responsive : services et bases côte à côte en fenêtre large.
    components::tile_grid(ui, p, 360.0, 2, |ui, p, i| match i {
        0 => services_card(app, ui, p),
        _ => db_card(app, ui, p),
    });
    ui.add_space(12.0);
    scan_card(app, ui, p);
}

// ─── Services ClamAV ─────────────────────────────────────────────────────────
fn services_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    let mut refresh = false;
    let mut restart_daemon = false;
    let mut stop_daemon = false;
    let mut start_daemon = false;
    let mut restart_fresh = false;
    let mut stop_fresh = false;
    let mut start_fresh = false;

    // Captures avant rendu (emprunts isolés).
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
            if components::icon_button(ui, p, crate::icons::REFRESH, t.tooltip_refresh).clicked() {
                refresh = true;
            }
        },
        |ui| {
            // clamav-daemon (démon d'analyse).
            service_row(app, ui, p, "clamav-daemon", &daemon_state, app.clam.daemon_at, &mut restart_daemon, &mut stop_daemon, &mut start_daemon);
            ui.add_space(6.0);
            // clamav-freshclam (mise à jour des bases).
            service_row(app, ui, p, "clamav-freshclam", &freshclam_state, app.clam.freshclam_at, &mut restart_fresh, &mut stop_fresh, &mut start_fresh);
        },
    );

    if refresh {
        app.send_op(Pending::ClamServices, Op::ClamServicesState);
    }
    if restart_daemon {
        app.send_op(
            Pending::ClamService { unit: "clamav-daemon".into(), action: ServiceAction::Restart },
            Op::ClamServiceAction { unit: "clamav-daemon".into(), action: ServiceAction::Restart },
        );
    }
    if stop_daemon {
        app.modal = Some(Modal::Confirm(ConfirmData {
            title: t.clam_stop_title.to_string(),
            body: t.clam_stop_body.to_string(),
            confirm_label: t.btn_stop.to_string(),
            kind: ConfirmKind::StopClamav { unit: "clamav-daemon".into() },
        }));
    }
    if start_daemon {
        app.send_op(
            Pending::ClamService { unit: "clamav-daemon".into(), action: ServiceAction::Start },
            Op::ClamServiceAction { unit: "clamav-daemon".into(), action: ServiceAction::Start },
        );
    }
    if restart_fresh {
        app.send_op(
            Pending::ClamService { unit: "clamav-freshclam".into(), action: ServiceAction::Restart },
            Op::ClamServiceAction { unit: "clamav-freshclam".into(), action: ServiceAction::Restart },
        );
    }
    if stop_fresh {
        app.modal = Some(Modal::Confirm(ConfirmData {
            title: t.clam_stop_title.to_string(),
            body: t.clam_stop_body.to_string(),
            confirm_label: t.btn_stop.to_string(),
            kind: ConfirmKind::StopClamav { unit: "clamav-freshclam".into() },
        }));
    }
    if start_fresh {
        app.send_op(
            Pending::ClamService { unit: "clamav-freshclam".into(), action: ServiceAction::Start },
            Op::ClamServiceAction { unit: "clamav-freshclam".into(), action: ServiceAction::Start },
        );
    }
}

/// Une ligne de service : point d'état + nom + badge + actions.
#[allow(clippy::too_many_arguments)]
fn service_row(
    app: &mut GuiApp,
    ui: &mut egui::Ui,
    p: &Palette,
    name: &str,
    state: &str,
    at: Option<std::time::Instant>,
    restart: &mut bool,
    stop: &mut bool,
    start: &mut bool,
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

    // Flux replié : état à gauche, actions dessous quand la largeur manque —
    // les boutons ne sortent jamais de l'écran.
    ui.vertical(|ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 6.0);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
            ui.painter().circle_filled(rect.center(), 5.0, col);
            ui.label(RichText::new(name).monospace());
            let bg = crate::components::blend(col, ui.style().visuals.panel_fill, 0.18);
            components::badge(ui, p, state, col, bg);
            if let Some(at) = at {
                components::since_label(ui, p, t, at);
            }
        });
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
            if active {
                if components::ghost_button(ui, p, t.btn_restart).clicked() {
                    *restart = true;
                }
                if components::ghost_button(ui, p, t.btn_stop).clicked() {
                    *stop = true;
                }
            } else if app.clam.daemon.is_some() {
                if components::primary_button(ui, p, t.btn_start).clicked() {
                    *start = true;
                }
            }
            if busy {
                ui.add(egui::Spinner::new().size(14.0));
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
            if components::icon_button(ui, p, crate::icons::REFRESH, t.tooltip_refresh).clicked() {
                refresh = true;
            }
            if components::primary_button_small(ui, p, crate::icons::REFRESH, t.clam_update_btn)
                .clicked()
            {
                update = true;
            }
        },
        |ui| {
            match &db {
                Some(db) => {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 6.0);
                        ui.label(RichText::new(t.clam_db_engine).size(12.5).color(p.text_secondary));
                        ui.label(RichText::new(&db.engine).monospace());
                        ui.label(RichText::new(t.clam_db_sigs).size(12.5).color(p.text_secondary));
                        ui.label(RichText::new(db.sigs.to_string()).monospace());
                        if let Some(at) = db_at {
                            components::since_label(ui, p, t, at);
                        }
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 6.0);
                        ui.label(RichText::new(t.clam_db_date).size(12.5).color(p.text_secondary));
                        ui.label(RichText::new(&db.db_date).monospace());
                    });
                }
                None => {
                    components::hint(ui, p, t.clam_db_unknown);
                }
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                if ui
                    .add(egui::Button::new(components::btn_icon_text(
                        crate::icons::HISTORY,
                        t.clam_logs_daemon_btn,
                    )))
                    .clicked()
                {
                    logs_daemon = true;
                }
                if ui
                    .add(egui::Button::new(components::btn_icon_text(
                        crate::icons::HISTORY,
                        t.clam_logs_freshclam_btn,
                    )))
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
            Op::ClamLogs { which: ClamLogSource::Daemon, lines: 300 },
        );
    }
    if logs_freshclam {
        app.send_op(
            Pending::ClamLogs,
            Op::ClamLogs { which: ClamLogSource::Freshclam, lines: 300 },
        );
    }
}

// ─── Analyse à la demande ────────────────────────────────────────────────────
fn scan_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    let mut scan = false;
    let busy = app.pending.values().any(|x| matches!(x, Pending::ClamScan));
    let daemon_active = app.clam.daemon.as_deref() == Some("active");

    // Le sélecteur de dossier n'a de sens qu'en local : en SSH, la cible
    // est distante et seul le chemin saisi à la main s'applique.
    let local_target = !app.conn_desc.starts_with("SSH");

    components::card(ui, p, t.clam_scan_title, |_| {}, |ui| {
        ui.label(RichText::new(t.clam_scan_path_label).size(12.5).color(p.text_secondary));
        // Rangée adaptative : champ + boutons sur une ligne en fenêtre
        // large ; champ pleine largeur puis boutons dessous en fenêtre
        // étroite.
        let avail = ui.available_width();
        let inline = avail >= 620.0;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            let resp = ui.add(
                egui::TextEdit::singleline(&mut app.clam.scan_path)
                    .hint_text(t.clam_scan_hint)
                    // Réserve la place des boutons et du spinner.
                    .desired_width(if inline {
                        (avail - 350.0).max(160.0)
                    } else {
                        (ui.available_width() - 8.0).max(160.0)
                    })
                    .font(egui::TextStyle::Monospace),
            );
            let path_ok = {
                let s = app.clam.scan_path.trim();
                s.starts_with('/') && !s.starts_with("/proc") && !s.starts_with("/sys")
                    && !s.starts_with("/dev") && !s.starts_with("/run")
            };
            let enabled = path_ok && !busy && !app.clam.picking;
            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && enabled {
                scan = true;
            }
            // Sélecteur de dossier natif (portail xdg), barre conservée.
            if components::styled_button_en(
                ui,
                p,
                &format!("{}  {}", crate::icons::FOLDER, t.clam_browse_btn),
                components::BtnKind::Soft,
                local_target && !app.clam.picking,
            )
            .clicked()
            {
                app.clam.picking = true;
                app.worker.send(crate::worker::Job::PickFolder);
            }
            // Bouton d'analyse large ; pendant le travail, la loupe est
            // remplacée par un spinner peint DANS le bouton.
            let label = if busy {
                format!("    {}", t.clam_scan_btn)
            } else {
                format!("{}  {}", crate::icons::SEARCH, t.clam_scan_btn)
            };
            let busy_btn = components::styled_button_minw(
                ui,
                p,
                &label,
                components::BtnKind::Primary,
                enabled,
                170.0,
            )
            .on_hover_text(t.clam_scan_hint);
            if busy {
                let r = busy_btn.rect;
                egui::Spinner::new().size(16.0).paint_at(
                    ui,
                    egui::Rect::from_min_size(
                        egui::pos2(r.left() + 16.0, r.center().y - 8.0),
                        egui::vec2(16.0, 16.0),
                    ),
                );
            }
            if busy_btn.clicked() {
                scan = true;
            }
        });
        ui.add_space(4.0);
        components::hint(ui, p, t.clam_scan_daemon_note);
    });

    if scan && !busy {
        let path = app.clam.scan_path.trim().to_string();
        app.send_op(Pending::ClamScan, Op::ClamScan { path, use_daemon: daemon_active });
    }
}
