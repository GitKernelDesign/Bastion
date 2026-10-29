//! Écran Fail2Ban : service, prisons, IP bannies, bannissement manuel.

use crate::app::{ConfirmData, ConfirmKind, GuiApp, Modal, Pending};
use crate::components;
use crate::theme::Palette;
use bastion_core::fail2ban::humanize_duration;
use bastion_core::model::{JailInfo, LogSource};
use bastion_core::op::{Op, ServiceAction};
use egui::{Align, Layout, RichText};

pub fn render(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    components::screen_header(ui, p, t.nav_fail2ban, t.f2b_subtitle);
    ui.add_space(6.0);

    if app.phase != crate::app::ConnPhase::Connected {
        components::empty_state(ui, p, t.status_disconnected, t.need_connection_hint);
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

// ─── Carte service ───────────────────────────────────────────────────────────
fn service_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    components::card_body_w(ui, p, 310.0, |ui| {
        let state = app.f2b.service.clone().unwrap_or_else(|| t.state_unknown.to_string());
        let active = state == "active";
        let color = if active {
            p.success
        } else if state == "failed" {
            p.danger
        } else {
            p.text_secondary
        };
        components::status_line(ui, p, color, t.service_label, Some(&state));
        if let Some(at) = app.f2b.service_at {
            components::since_label(ui, p, t, at);
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if active {
                if ui
                    .add(egui::Button::new(components::btn_icon_text(crate::icons::RESTART, t.btn_restart)))
                    .clicked()
                {
                    app.modal = Some(Modal::Confirm(ConfirmData {
                        title: t.restart_f2b_title.into(),
                        body: t.restart_f2b_body.into(),
                        confirm_label: t.btn_restart.into(),
                        kind: ConfirmKind::RestartF2b,
                    }));
                }
                if ui
                    .add(egui::Button::new(components::btn_icon_text(crate::icons::STOP, t.btn_stop)))
                    .clicked()
                {
                    app.modal = Some(Modal::Confirm(ConfirmData {
                        title: t.stop_f2b_title.into(),
                        body: t.stop_f2b_body.into(),
                        confirm_label: t.btn_stop.into(),
                        kind: ConfirmKind::StopF2b,
                    }));
                }
            } else if components::primary_button(ui, p, t.btn_start).clicked() {
                app.send_op(
                    Pending::Service(ServiceAction::Start),
                    Op::F2bServiceAction { action: ServiceAction::Start },
                );
            }
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                if ui
                    .add(egui::Button::new(components::btn_icon_text(crate::icons::REFRESH, t.reload_btn)))
                    .on_hover_text(t.reload_service_tooltip)
                    .clicked()
                {
                    app.send_op(Pending::ReloadJail, Op::F2bReloadJail { jail: None });
                }
            });
        });
    });
}

// ─── Carte synthèse ──────────────────────────────────────────────────────────
fn summary_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    components::card_body_w(ui, p, 220.0, |ui| {
        let (jails, banned) = match &app.f2b.global {
            Some(g) => {
                let b: u64 = g
                    .jails
                    .iter()
                    .filter_map(|j| app.f2b.jail_infos.get(j).map(|(info, _)| info.currently_banned.unwrap_or(0)))
                    .sum();
                (g.jails.len(), b)
            }
            None => (0, 0),
        };
        ui.horizontal(|ui| {
            big_number(ui, p, jails.to_string(), t.jails_count_label);
            ui.add_space(16.0);
            big_number(ui, p, banned.to_string(), t.banned_count_label);
        });
        if let Some(at) = app.f2b.global_at {
            ui.add_space(4.0);
            components::since_label(ui, p, t, at);
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

// ─── Carte prisons ───────────────────────────────────────────────────────────
fn jails_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    let jails: Vec<String> = app
        .f2b
        .global
        .as_ref()
        .map(|g| g.jails.clone())
        .unwrap_or_default();
    let mut refresh_clicked = false;
    let mut logs_clicked = false;
    let mut banned_all_clicked = false;

    components::card(
        ui,
        p,
        &t.jails_with_count(jails.len()),
        |ui| {
            if components::icon_button(ui, p, crate::icons::REFRESH, t.refresh_rules_tooltip).clicked() {
                refresh_clicked = true;
            }
            if ui
                .add(egui::Button::new(components::btn_icon_text(crate::icons::BAN, t.all_ips_btn)))
                .on_hover_text(t.all_ips_tooltip)
                .clicked()
            {
                banned_all_clicked = true;
            }
            if ui
                .add(egui::Button::new(components::btn_icon_text(crate::icons::TERMINAL, t.service_log_btn)))
                .on_hover_text(t.service_log_tooltip)
                .clicked()
            {
                logs_clicked = true;
            }
        },
        |ui| {
            if jails.is_empty() {
                components::empty_state(ui, p, t.jails_empty, t.jails_empty_hint);
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
                            ui.label(header(p, t.col_jail));
                            ui.label(header(p, t.col_banned));
                            ui.label(header(p, t.col_failed));
                            ui.label(header(p, "bantime"));
                            ui.label(header(p, "findtime"));
                            ui.label(header(p, ""));
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
                                    app.send_op(Pending::RefreshJail, Op::F2bJail { jail: jail.clone() });
                                }
                                if let Some((info, _)) = app.f2b.jail_infos.get(jail) {
                                    ui.label(opt_u64(info.currently_banned));
                                    ui.label(opt_u64(info.currently_failed));
                                    ui.label(
                                        RichText::new(
                                            info.bantime.as_deref().map(humanize_duration).unwrap_or_default(),
                                        )
                                        .size(12.5),
                                    );
                                    ui.label(
                                        RichText::new(
                                            info.findtime.as_deref().map(humanize_duration).unwrap_or_default(),
                                        )
                                        .size(12.5),
                                    );
                                } else {
                                    ui.label(RichText::new("…").weak());
                                    ui.label(RichText::new("…").weak());
                                    ui.label(RichText::new("…").weak());
                                    ui.label(RichText::new("…").weak());
                                }
                                ui.label(RichText::new(""));
                                ui.end_row();
                            }
                        });
                });
        },
    );

    if refresh_clicked {
        for jail in jails {
            app.send_op(Pending::RefreshJail, Op::F2bJail { jail });
        }
    }
    if banned_all_clicked {
        app.send_op(Pending::BannedAll, Op::F2bBannedAll);
    }
    if logs_clicked {
        app.send_op(
            Pending::Logs,
            Op::F2bLogs { which: LogSource::Fail2ban, lines: 300 },
        );
    }
}

// ─── Carte prison sélectionnée ───────────────────────────────────────────────
fn jail_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette, jail: &str) {
    let t = app.tr;
    let (info, at) = match app.f2b.jail_infos.get(jail) {
        Some((i, t_at)) => (i.clone(), *t_at),
        None => (JailInfo::default(), std::time::Instant::now()),
    };
    let mut refresh_clicked = false;
    let mut reload_clicked = false;

    components::card(
        ui,
        p,
        &t.jail_title(jail),
        |ui| {
            if components::icon_button(ui, p, crate::icons::REFRESH, t.jail_refresh_tooltip).clicked() {
                refresh_clicked = true;
            }
            if ui
                .add(egui::Button::new(components::btn_icon_text(crate::icons::RESTART, t.reload_btn)))
                .on_hover_text(t.jail_reload_tooltip)
                .clicked()
            {
                reload_clicked = true;
            }
        },
        |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(t.current_failures_label).size(12.5).color(p.text_secondary));
                ui.label(opt_u64(info.currently_failed));
                ui.label(RichText::new(t.total_label).size(12.5).color(p.text_secondary));
                ui.label(opt_u64(info.total_failed));
                ui.label(RichText::new(t.maxretry_label).size(12.5).color(p.text_secondary));
                ui.label(
                    RichText::new(info.maxretry.clone().unwrap_or_else(|| "—".into())).monospace(),
                );
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    components::since_label(ui, p, t, at);
                });
            });
            if let Some(files) = &info.file_list {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(t.watches_label).size(12.5).color(p.text_secondary));
                    ui.label(RichText::new(components::ellipsize(files, 60)).monospace())
                        .on_hover_text(files);
                });
            }
            ui.add_space(8.0);

            // Bannissement manuel.
            ui.horizontal(|ui| {
                let ok_ip = bastion_core::ufw::is_valid_addr(app.f2b.ban_input.trim());
                let edit = egui::TextEdit::singleline(&mut app.f2b.ban_input)
                    .hint_text(t.ban_input_hint)
                    .desired_width(240.0)
                    .font(egui::TextStyle::Monospace);
                let resp = ui.add(edit);
                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && ok_ip {
                    ban(app, jail);
                }
                if ui
                    .add_enabled(
                        ok_ip,
                        egui::Button::new(components::btn_icon_text(crate::icons::BAN, t.btn_ban)),
                    )
                    .clicked()
                {
                    ban(app, jail);
                }
            });
            ui.add_space(8.0);

            ui.label(
                RichText::new(format!("{} ({})", t.banned_list_title, info.banned_ips.len()))
                    .size(11.5)
                    .weak()
                    .family(egui::FontFamily::Name("Medium".into())),
            );
            ui.add_space(4.0);
            if info.banned_ips.is_empty() {
                components::hint(ui, p, t.no_banned);
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
                                        t.btn_unban,
                                    )))
                                    .clicked()
                                {
                                    app.send_op(
                                        Pending::Unban { jail: jail.to_string(), ip: ip.clone() },
                                        Op::F2bUnbanIp { jail: jail.to_string(), ip: ip.clone() },
                                    );
                                }
                                ui.end_row();
                            }
                        });
                });
        },
    );

    if refresh_clicked {
        app.send_op(Pending::RefreshJail, Op::F2bJail { jail: jail.to_string() });
    }
    if reload_clicked {
        app.send_op(
            Pending::ReloadJail,
            Op::F2bReloadJail { jail: Some(jail.to_string()) },
        );
    }
}

fn ban(app: &mut GuiApp, jail: &str) {
    let ip = app.f2b.ban_input.trim().to_string();
    app.f2b.ban_input.clear();
    app.send_op(
        Pending::Ban { jail: jail.to_string(), ip: ip.clone() },
        Op::F2bBanIp { jail: jail.to_string(), ip },
    );
}

fn opt_u64(v: Option<u64>) -> RichText {
    RichText::new(v.map(|x| x.to_string()).unwrap_or_else(|| "—".into())).monospace()
}

fn header(p: &Palette, s: &str) -> RichText {
    RichText::new(s)
        .size(11.5)
        .weak()
        .family(egui::FontFamily::Name("Medium".into()))
        .color(p.text_secondary)
}