//! Écran Fail2Ban : synthèse, service, prisons et adresses bannies.
//!
//! Disposition maître-détail : la liste des prisons à gauche, le détail de la
//! prison sélectionnée à droite (empilés en fenêtre étroite).

use crate::app::{ConfirmData, ConfirmKind, GuiApp, Modal, Pending};
use crate::components;
use crate::icons;
use crate::theme::{space, text, Palette};
use bastion_core::fail2ban::humanize_duration;
use bastion_core::model::{JailInfo, LogSource};
use bastion_core::op::{Op, ServiceAction};
use egui::{Align, CornerRadius, FontId, Layout, RichText, Sense, Stroke};

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

    stats_row(app, ui, p);
    ui.add_space(space::M);
    service_card(app, ui, p);
    ui.add_space(space::M);

    let jails = app
        .f2b
        .global
        .as_ref()
        .map(|g| g.jails.len())
        .unwrap_or(0);
    let wide = ui.available_width() >= 760.0;
    if wide && jails > 0 {
        // Deux colonnes : liste (40 %) et détail (60 %).
        let total = ui.available_width();
        let list_w = (total * 0.4 - 6.0).max(260.0);
        let h = ui.available_height();
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(list_w, h),
                Layout::top_down(Align::Min),
                |ui| jails_card(app, ui, p),
            );
            ui.add_space(space::M);
            let rest = ui.available_width();
            ui.allocate_ui_with_layout(
                egui::vec2(rest, h),
                Layout::top_down(Align::Min),
                |ui| detail_card(app, ui, p),
            );
        });
    } else {
        jails_card(app, ui, p);
        ui.add_space(space::M);
        detail_card(app, ui, p);
    }
}

fn jail_banned_sum(app: &GuiApp) -> u64 {
    app.f2b
        .jail_infos
        .values()
        .map(|(i, _)| i.currently_banned.unwrap_or(0))
        .sum()
}

fn jail_failed_sum(app: &GuiApp) -> u64 {
    app.f2b
        .jail_infos
        .values()
        .map(|(i, _)| i.currently_failed.unwrap_or(0))
        .sum()
}

// ─── Chiffres clés ───────────────────────────────────────────────────────────
fn stats_row(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    let jails = app
        .f2b
        .global
        .as_ref()
        .map(|g| g.jails.len())
        .unwrap_or(0);
    let banned = jail_banned_sum(app);
    let failed = jail_failed_sum(app);
    components::tile_grid(ui, p, 150.0, 3, |ui, p, i| match i {
        0 => components::stat_tile(
            ui,
            p,
            icons::SHIELD,
            p.accent,
            &jails.to_string(),
            t.jails_count_label,
        ),
        1 => components::stat_tile(
            ui,
            p,
            icons::BAN,
            p.danger,
            &banned.to_string(),
            t.banned_count_label,
        ),
        _ => components::stat_tile(
            ui,
            p,
            icons::TIMER,
            p.warning,
            &failed.to_string(),
            t.f2b_stat_failed,
        ),
    });
}

// ─── Service ─────────────────────────────────────────────────────────────────
fn service_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    let mut logs = false;
    let mut banned_all = false;
    let (mut hdr_logs, mut hdr_all) = (false, false);
    let mut reload = false;
    let mut restart = false;
    let mut stop = false;
    let mut start = false;
    let state = app
        .f2b
        .service
        .clone()
        .unwrap_or_else(|| t.state_unknown.to_string());
    let active = state == "active";
    let color = if active {
        p.success
    } else if state == "failed" {
        p.danger
    } else {
        p.text_secondary
    };

    components::card(
        ui,
        p,
        t.f2b_service_title,
        |ui| {
            if let Some(at) = app.f2b.service_at {
                components::since_label(ui, p, t, at);
            }
            if components::icon_button(ui, p, icons::TERMINAL, t.service_log_tooltip).clicked() {
                hdr_logs = true;
            }
            if components::icon_button(ui, p, icons::LIST, t.all_ips_tooltip).clicked() {
                hdr_all = true;
            }
        },
        |ui| {
            ui.horizontal_wrapped(|ui| {
                components::status_dot(ui, color, 8.0);
                ui.label(
                    RichText::new(t.service_label)
                        .size(text::BODY)
                        .color(p.text)
                        .family(egui::FontFamily::Name("Medium".into())),
                );
                components::tag(ui, p, &state, color);
            });
            ui.add_space(space::S);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
                if active {
                    if components::ghost_button(
                        ui,
                        p,
                        &format!("{}  {}", icons::RESTART, t.btn_restart),
                    )
                    .clicked()
                    {
                        restart = true;
                    }
                    if components::ghost_button(ui, p, t.btn_stop).clicked() {
                        stop = true;
                    }
                    if components::soft_button(
                        ui,
                        p,
                        &format!("{}  {}", icons::SYNC, t.reload_btn),
                    )
                    .on_hover_text(t.reload_service_tooltip)
                    .clicked()
                    {
                        reload = true;
                    }
                } else if components::primary_button(ui, p, t.btn_start).clicked() {
                    start = true;
                }
                let (mut a, mut b) = (false, false);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    a = components::link_button(ui, p, t.all_ips_btn).clicked();
                    b = components::link_button(ui, p, t.service_log_btn).clicked();
                });
                banned_all |= a;
                logs |= b;
            });
        },
    );

    logs |= hdr_logs;
    banned_all |= hdr_all;
    if logs {
        app.send_op(
            Pending::Logs,
            Op::F2bLogs {
                which: LogSource::Fail2ban,
                lines: 300,
            },
        );
    }
    if banned_all {
        app.send_op(Pending::BannedAll, Op::F2bBannedAll);
    }
    if reload {
        app.send_op(Pending::ReloadJail, Op::F2bReloadJail { jail: None });
    }
    if restart {
        app.modal = Some(Modal::Confirm(ConfirmData {
            title: t.restart_f2b_title.into(),
            body: t.restart_f2b_body.into(),
            confirm_label: t.btn_restart.into(),
            kind: ConfirmKind::RestartF2b,
        }));
    }
    if stop {
        app.modal = Some(Modal::Confirm(ConfirmData {
            title: t.stop_f2b_title.into(),
            body: t.stop_f2b_body.into(),
            confirm_label: t.btn_stop.into(),
            kind: ConfirmKind::StopF2b,
        }));
    }
    if start {
        app.send_op(
            Pending::Service(ServiceAction::Start),
            Op::F2bServiceAction {
                action: ServiceAction::Start,
            },
        );
    }
}

// ─── Liste des prisons ───────────────────────────────────────────────────────
fn jails_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    let jails: Vec<String> = app
        .f2b
        .global
        .as_ref()
        .map(|g| g.jails.clone())
        .unwrap_or_default();
    let mut refresh_clicked = false;

    let filter = app.f2b.jail_filter.to_lowercase();
    let shown: Vec<String> = jails
        .iter()
        .filter(|j| filter.is_empty() || j.to_lowercase().contains(&filter))
        .cloned()
        .collect();

    components::card(
        ui,
        p,
        &t.jails_with_count(jails.len()),
        |ui| {
            if components::icon_button(ui, p, icons::REFRESH, t.refresh_rules_tooltip).clicked() {
                refresh_clicked = true;
            }
        },
        |ui| {
            if jails.len() > 2 {
                if components::search_field(ui, p, &mut app.f2b.jail_filter, t.f2b_search_jails)
                    .changed()
                {
                    // Rien de plus : le filtre s'applique au rendu suivant.
                }
                ui.add_space(space::S);
            }
            if jails.is_empty() {
                components::empty_state(ui, p, t.jails_empty, t.jails_empty_hint);
                return;
            }
            if shown.is_empty() {
                components::empty_state(ui, p, t.f2b_no_jail_match, t.fw_no_match_hint);
                return;
            }

            // En-têtes de colonnes, alignés sur les chiffres des lignes.
            let (hdr, _) =
                ui.allocate_exact_size(egui::vec2(ui.available_width(), 18.0), Sense::hover());
            let cy = hdr.center().y;
            paint_left(ui, t.col_jail, th_font(), p.text_secondary, hdr.left() + 10.0, cy);
            paint_right(
                ui,
                t.col_failed,
                th_font(),
                p.text_secondary,
                col_failed_right(hdr.right()),
                cy,
            );
            paint_right(
                ui,
                t.col_banned,
                th_font(),
                p.text_secondary,
                col_banned_right(hdr.right()),
                cy,
            );
            ui.add_space(2.0);
            components::hairline(ui, p);
            ui.add_space(2.0);

            let mut clicked: Option<String> = None;
            for jail in &shown {
                let info = app.f2b.jail_infos.get(jail);
                let banned = info.and_then(|(i, _)| i.currently_banned).unwrap_or(0);
                let failed = info.and_then(|(i, _)| i.currently_failed).unwrap_or(0);
                let selected = app.f2b.selected_jail.as_deref() == Some(jail.as_str());
                if jail_row(ui, p, jail, banned, failed, selected).clicked() {
                    clicked = Some(jail.clone());
                }
            }
            if let Some(jail) = clicked {
                app.f2b.selected_jail = Some(jail.clone());
                app.send_op(Pending::RefreshJail, Op::F2bJail { jail });
            }
        },
    );

    if refresh_clicked {
        for jail in jails {
            app.send_op(Pending::RefreshJail, Op::F2bJail { jail });
        }
    }
}

/// Ligne de prison cliquable : nom, bannies, échecs, chevron.
fn jail_row(
    ui: &mut egui::Ui,
    p: &Palette,
    jail: &str,
    banned: u64,
    failed: u64,
    selected: bool,
) -> egui::Response {
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 38.0), Sense::click());
    let fill = if selected {
        p.soft(p.accent)
    } else if resp.hovered() {
        p.card_hover
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter().rect(
        rect,
        CornerRadius::same(8),
        fill,
        Stroke::NONE,
        egui::StrokeKind::Inside,
    );
    let fg = if selected { p.accent } else { p.text };
    let name = ui.painter().layout_no_wrap(
        components::ellipsize(jail, 30),
        FontId::new(text::LABEL, egui::FontFamily::Name("MonoMedium".into())),
        fg,
    );
    ui.painter().galley(
        egui::pos2(rect.left() + 10.0, rect.center().y - name.size().y * 0.5),
        name,
        fg,
    );

    // Colonnes de chiffres alignées à droite, chevron à l'extrémité.
    let cy = rect.center().y;
    let num_font = FontId::new(text::MONO, egui::FontFamily::Name("MonoMedium".into()));
    paint_right(
        ui,
        &failed.to_string(),
        num_font.clone(),
        p.warning,
        col_failed_right(rect.right()),
        cy,
    );
    paint_right(
        ui,
        &banned.to_string(),
        num_font,
        if banned > 0 { p.danger } else { p.text_secondary },
        col_banned_right(rect.right()),
        cy,
    );
    let chevron = ui.painter().layout_no_wrap(
        icons::CHEVRON_RIGHT.to_string(),
        FontId::new(14.0, egui::FontFamily::Name("Icon".into())),
        p.text_secondary,
    );
    ui.painter().galley(
        egui::pos2(
            rect.right() - JAIL_PAD - chevron.size().x,
            cy - chevron.size().y * 0.5,
        ),
        chevron,
        p.text_secondary,
    );
    resp
}

// Colonnes du tableau des prisons : mêmes repères pour l'en-tête et les lignes.
const JAIL_PAD: f32 = 10.0;
const JAIL_W_CHEVRON: f32 = 24.0;
const JAIL_W_COL: f32 = 56.0;

fn col_failed_right(row_right: f32) -> f32 {
    row_right - JAIL_PAD - JAIL_W_CHEVRON - JAIL_W_COL
}

fn col_banned_right(row_right: f32) -> f32 {
    col_failed_right(row_right) - JAIL_W_COL
}

fn th_font() -> FontId {
    FontId::new(
        text::MICRO,
        egui::FontFamily::Name("Medium".into()),
    )
}

/// Texte aligné à droite sur `right_x`.
fn paint_right(
    ui: &mut egui::Ui,
    text: &str,
    font: FontId,
    color: egui::Color32,
    right_x: f32,
    cy: f32,
) {
    let g = ui.painter().layout_no_wrap(text.to_string(), font, color);
    let pos = egui::pos2(right_x - g.size().x, cy - g.size().y * 0.5);
    ui.painter().galley(pos, g, color);
}

/// Texte aligné à gauche en `left_x`.
fn paint_left(
    ui: &mut egui::Ui,
    text: &str,
    font: FontId,
    color: egui::Color32,
    left_x: f32,
    cy: f32,
) {
    let g = ui.painter().layout_no_wrap(text.to_string(), font, color);
    let pos = egui::pos2(left_x, cy - g.size().y * 0.5);
    ui.painter().galley(pos, g, color);
}

// ─── Détail d'une prison ─────────────────────────────────────────────────────
fn detail_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    let Some(jail) = app.f2b.selected_jail.clone() else {
        components::card_body(ui, p, |ui| {
            components::empty_state(ui, p, t.f2b_pick_jail, t.f2b_pick_jail_hint);
        });
        return;
    };
    let (info, at) = match app.f2b.jail_infos.get(&jail).cloned() {
        Some((i, at)) => (i, at),
        None => (JailInfo::default(), std::time::Instant::now()),
    };
    let mut refresh_clicked = false;
    let mut reload_clicked = false;
    let mut copy_clicked = false;
    let mut ban_clicked = false;
    let mut unban: Option<String> = None;

    components::card(
        ui,
        p,
        &t.jail_title(&jail),
        |ui| {
            components::since_label(ui, p, t, at);
            if components::icon_button(ui, p, icons::REFRESH, t.jail_refresh_tooltip).clicked() {
                refresh_clicked = true;
            }
            if components::icon_button(ui, p, icons::SYNC, t.jail_reload_tooltip).clicked() {
                reload_clicked = true;
            }
        },
        |ui| {
            // Paramètres de la prison.
            egui::Grid::new("jail-params")
                .num_columns(4)
                .spacing(egui::vec2(10.0, 7.0))
                .show(ui, |ui| {
                    kv_pair(ui, p, t.current_failures_label, info.currently_failed);
                    kv_pair(ui, p, t.total_label, info.total_failed);
                    ui.end_row();
                    kv_text(ui, p, t.maxretry_label, info.maxretry.clone());
                    kv_text(ui, p, "bantime", info.bantime.as_deref().map(humanize_duration));
                    ui.end_row();
                    kv_text(ui, p, "findtime", info.findtime.as_deref().map(humanize_duration));
                    ui.label("");
                    ui.end_row();
                });
            if let Some(files) = &info.file_list {
                ui.add_space(2.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(t.watches_label)
                            .size(text::SMALL)
                            .color(p.text_secondary),
                    );
                    ui.label(RichText::new(components::ellipsize(files, 62)).monospace())
                        .on_hover_text(files);
                });
            }

            ui.add_space(space::M);
            components::hairline(ui, p);
            ui.add_space(space::M);

            // Bannissement manuel.
            let ok_ip = bastion_core::ufw::is_valid_addr(app.f2b.ban_input.trim());
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 6.0);
                let btn_w = 118.0;
                let edit = egui::TextEdit::singleline(&mut app.f2b.ban_input)
                    .hint_text(t.ban_input_hint)
                    .desired_width((ui.available_width() - btn_w - 10.0).clamp(140.0, 260.0))
                    .margin(egui::Margin::symmetric(10, 8))
                    .font(egui::TextStyle::Monospace);
                let resp = ui.add(edit);
                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && ok_ip {
                    ban_clicked = true;
                }
                let btn = components::styled_button_en(
                    ui,
                    p,
                    &format!("{}  {}", icons::BAN, t.btn_ban),
                    components::BtnKind::Primary,
                    ok_ip,
                );
                if btn.clicked() {
                    ban_clicked = true;
                }
                if !app.f2b.ban_input.trim().is_empty() && !ok_ip {
                    ui.label(
                        RichText::new(t.f2b_ban_invalid)
                            .size(text::MICRO)
                            .color(p.danger),
                    );
                }
            });
            ui.add_space(space::M);

            // Adresses bannies.
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{} ({})",
                        t.banned_list_title,
                        info.banned_ips.len()
                    ))
                    .size(text::MICRO)
                    .color(p.text_secondary)
                    .family(egui::FontFamily::Name("Medium".into())),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if !info.banned_ips.is_empty()
                        && components::icon_button(ui, p, icons::COPY, t.f2b_copy_ips).clicked()
                    {
                        copy_clicked = true;
                    }
                });
            });
            ui.add_space(space::XS);

            if info.banned_ips.is_empty() {
                components::hint(ui, p, t.no_banned);
                return;
            }
            let h = (30.0 + 30.0 * info.banned_ips.len() as f32).clamp(90.0, 340.0);
            egui::ScrollArea::vertical()
                .id_salt("banned-list")
                .max_height(h)
                .min_scrolled_height(h)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for ip in &info.banned_ips {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(ip)
                                    .monospace()
                                    .family(egui::FontFamily::Name("MonoMedium".into())),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if components::icon_button(
                                    ui,
                                    p,
                                    icons::RESTORE,
                                    t.btn_unban,
                                )
                                .clicked()
                                {
                                    unban = Some(ip.clone());
                                }
                                components::stretch_cell(ui);
                            });
                        });
                    }
                });
        },
    );

    if refresh_clicked {
        app.send_op(Pending::RefreshJail, Op::F2bJail { jail: jail.clone() });
    }
    if reload_clicked {
        app.send_op(
            Pending::ReloadJail,
            Op::F2bReloadJail {
                jail: Some(jail.clone()),
            },
        );
    }
    if copy_clicked {
        let text = app
            .f2b
            .jail_infos
            .get(&jail)
            .map(|(i, _)| i.banned_ips.join("\n"))
            .unwrap_or_default();
        ui.ctx().copy_text(text);
        app.toasts.info(app.tr.f2b_ips_copied.to_string());
    }
    if ban_clicked {
        let ip = app.f2b.ban_input.trim().to_string();
        app.f2b.ban_input.clear();
        app.send_op(
            Pending::Ban {
                jail: jail.clone(),
                ip: ip.clone(),
            },
            Op::F2bBanIp {
                jail: jail.clone(),
                ip,
            },
        );
    }
    if let Some(ip) = unban {
        app.send_op(
            Pending::Unban {
                jail: jail.clone(),
                ip: ip.clone(),
            },
            Op::F2bUnbanIp { jail, ip },
        );
    }
}

fn kv_pair(ui: &mut egui::Ui, p: &Palette, label: &str, value: Option<u64>) {
    ui.label(RichText::new(label).size(text::SMALL).color(p.text_secondary));
    ui.label(
        RichText::new(value.map(|v| v.to_string()).unwrap_or_else(|| "—".into()))
            .monospace()
            .family(egui::FontFamily::Name("MonoMedium".into())),
    );
}

fn kv_text(ui: &mut egui::Ui, p: &Palette, label: &str, value: Option<String>) {
    ui.label(RichText::new(label).size(text::SMALL).color(p.text_secondary));
    ui.label(
        RichText::new(value.as_deref().unwrap_or("—"))
            .monospace()
            .family(egui::FontFamily::Name("MonoMedium".into())),
    );
}
