//! Écran Journal : toutes les commandes exécutées, filtrables et copiables.
//! Une ligne = une commande réelle, avec son code de retour.

use crate::app::{ConfirmData, ConfirmKind, GuiApp, JOURNAL_CAP, LogFilter, Modal};
use crate::components;
use crate::icons;
use crate::theme::{space, text, Palette};
use egui::{Align, Layout, RichText};
use std::time::SystemTime;

pub fn render(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;

    // Compteurs par filtre (calculés une fois, avant le rendu).
    let n_all = app.journal.len();
    let n_err = app.journal.iter().filter(|e| !e.ok).count();
    let n_fw = app
        .journal
        .iter()
        .filter(|e| is_firewall_line(&e.command))
        .count();
    let n_f2b = app
        .journal
        .iter()
        .filter(|e| is_f2b_line(&e.command))
        .count();

    let mut copy = false;
    let mut clear = false;

    components::card_body(ui, p, |ui| {
        // Filtres (puces) + actions.
        let wide = ui.available_width() >= 620.0;
        if wide {
            ui.horizontal(|ui| {
                filter_chips(app, ui, p, n_all, n_err, n_fw, n_f2b);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if components::icon_button(ui, p, icons::DELETE, t.clear_btn).clicked() {
                        clear = true;
                    }
                    if components::styled_button(
                        ui,
                        p,
                        &format!("{}  {}", icons::COPY, t.journal_copy_view),
                        components::BtnKind::Ghost,
                    )
                    .on_hover_text(t.copy_tooltip)
                    .clicked()
                    {
                        copy = true;
                    }
                });
            });
        } else {
            ui.horizontal_wrapped(|ui| {
                filter_chips(app, ui, p, n_all, n_err, n_fw, n_f2b);
            });
            ui.add_space(space::S);
            ui.horizontal(|ui| {
                if components::styled_button(
                    ui,
                    p,
                    &format!("{}  {}", icons::COPY, t.journal_copy_view),
                    components::BtnKind::Ghost,
                )
                .on_hover_text(t.copy_tooltip)
                .clicked()
                {
                    copy = true;
                }
                if components::styled_button(
                    ui,
                    p,
                    &format!("{}  {}", icons::DELETE, t.clear_btn),
                    components::BtnKind::Quiet,
                )
                .clicked()
                {
                    clear = true;
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(t.entries_count(app.journal.len(), JOURNAL_CAP))
                            .size(text::MICRO)
                            .color(p.text_secondary),
                    );
                });
            });
        }
        ui.add_space(space::M);

        if app.journal.is_empty() {
            components::empty_state(ui, p, t.journal_empty, t.journal_empty_hint);
            return;
        }

        let mut row_copy: Option<String> = None;
        let h = ui.available_height().clamp(280.0, 520.0);
        let full_w = ui.available_width();
        // Colonnes à largeur fixe : la commande reste sur une ligne et le
        // tableau ne se déforme pas quand une commande est longue.
        let w_time = 62.0;
        let w_state = 20.0;
        let w_copy = 24.0;
        let show_detail = full_w >= 660.0;
        let w_detail = if show_detail {
            (full_w * 0.24).clamp(150.0, 280.0)
        } else {
            0.0
        };
        let gaps = 10.0 * (if show_detail { 4.0 } else { 3.0 });
        let w_cmd = (full_w - w_time - w_state - w_copy - w_detail - gaps).max(120.0);
        egui::ScrollArea::vertical()
            .id_salt("journal-scroll")
            .max_height(h)
            .min_scrolled_height(h)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // En-tête de table.
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(10.0, 0.0);
                    cell(ui, w_time, |ui| {
                        ui.label(components::th(p, t.journal_col_time));
                    });
                    cell(ui, w_state, |ui| {
                        ui.label(components::th(p, t.journal_col_status));
                    });
                    cell(ui, w_cmd, |ui| {
                        ui.label(components::th(p, t.journal_col_command));
                    });
                    if show_detail {
                        cell(ui, w_detail, |ui| {
                            ui.label(components::th(p, t.journal_col_detail));
                        });
                    }
                    ui.allocate_exact_size(egui::vec2(w_copy, 0.0), egui::Sense::hover());
                });
                ui.add_space(4.0);
                components::hairline(ui, p);
                ui.add_space(2.0);

                for entry in app.journal.iter().rev() {
                    if !matches_filter(app.filter, &entry.command, entry.ok) {
                        continue;
                    }
                    ui.add_space(5.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(10.0, 0.0);
                        cell(ui, w_time, |ui| {
                            ui.label(
                                RichText::new(fmt_time(entry.at))
                                    .size(text::MONO)
                                    .monospace()
                                    .color(p.text_secondary),
                            )
                            .on_hover_text(fmt_date(entry.at));
                        });
                        cell(ui, w_state, |ui| {
                            ui.label(
                                RichText::new(if entry.ok {
                                    icons::CHECK_CIRCLE
                                } else {
                                    icons::ERROR
                                })
                                .family(egui::FontFamily::Name("Icon".into()))
                                .size(15.0)
                                .color(if entry.ok { p.success } else { p.danger }),
                            )
                            .on_hover_text(if entry.ok {
                                format!("code {}", entry.code)
                            } else {
                                format!("code {}\n{}", entry.code, entry.detail)
                            });
                        });
                        cell(ui, w_cmd, |ui| {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(&entry.command)
                                        .monospace()
                                        .size(text::MONO)
                                        .color(p.text),
                                )
                                .truncate(),
                            )
                            .on_hover_text(&entry.command);
                        });
                        if show_detail {
                            cell(ui, w_detail, |ui| {
                                if entry.ok {
                                    ui.label(
                                        RichText::new(format!("code {}", entry.code))
                                            .size(text::MICRO)
                                            .color(p.text_secondary),
                                    );
                                } else {
                                    let first = entry.detail.lines().next().unwrap_or("");
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(first)
                                                .monospace()
                                                .size(text::MICRO)
                                                .color(p.danger),
                                        )
                                        .truncate(),
                                    )
                                    .on_hover_text(&entry.detail);
                                }
                            });
                        }
                        cell(ui, w_copy, |ui| {
                            if components::icon_button_sz(
                                ui,
                                p,
                                icons::COPY,
                                t.journal_row_copy,
                                24.0,
                                12.0,
                            )
                            .clicked()
                            {
                                row_copy = Some(entry.command.clone());
                            }
                        });
                    });
                }
                ui.add_space(4.0);
            });

        if let Some(cmd) = row_copy {
            ui.ctx().copy_text(cmd);
            app.toasts.info(t.journal_copied_toast.to_string());
        }
    });

    if copy {
        let text = app
            .journal
            .iter()
            .filter(|e| matches_filter(app.filter, &e.command, e.ok))
            .map(|e| {
                format!(
                    "{} {} {} code={}",
                    fmt_time(e.at),
                    if e.ok { "OK  " } else { "ERR " },
                    e.command,
                    e.code
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        ui.ctx().copy_text(text);
        app.toasts.info(t.journal_copied_toast.to_string());
    }
    if clear {
        app.modal = Some(Modal::Confirm(ConfirmData {
            title: t.journal_clear_title.to_string(),
            body: t.journal_clear_body.to_string(),
            confirm_label: t.clear_btn.to_string(),
            kind: ConfirmKind::ClearJournal,
        }));
    }
}

fn matches_filter(filter: LogFilter, command: &str, ok: bool) -> bool {
    match filter {
        LogFilter::All => true,
        LogFilter::Errors => !ok,
        LogFilter::Fw => is_firewall_line(command),
        LogFilter::F2b => is_f2b_line(command),
    }
}

/// Puces de filtre avec leur compteur.
fn filter_chips(
    app: &mut GuiApp,
    ui: &mut egui::Ui,
    p: &Palette,
    n_all: usize,
    n_err: usize,
    n_fw: usize,
    n_f2b: usize,
) {
    let t = app.tr;
    for (f, label, n) in [
        (LogFilter::All, t.filter_all, n_all),
        (LogFilter::Errors, t.filter_errors, n_err),
        (LogFilter::Fw, t.filter_fw, n_fw),
        (LogFilter::F2b, t.filter_f2b, n_f2b),
    ] {
        let selected = app.filter == f;
        if components::chip(ui, p, &format!("{label}  {n}"), selected).clicked() {
            app.filter = f;
        }
    }
}

fn is_firewall_line(cmd: &str) -> bool {
    cmd.starts_with("ufw") || cmd.starts_with("journalctl -k")
}

fn is_f2b_line(cmd: &str) -> bool {
    cmd.starts_with("fail2ban-client")
        || cmd.starts_with("systemctl")
        || cmd.starts_with("journalctl -u fail2ban")
}

/// Heure locale du poste (HH:MM:SS), lisible d'un coup d'œil.
fn fmt_time(t: SystemTime) -> String {
    let secs = epoch_secs(t);
    let offset = local_offset_secs();
    let local = secs as i64 + offset;
    let rem = local.rem_euclid(86_400) as u64;
    format!("{:02}:{:02}:{:02}", rem / 3600, (rem % 3600) / 60, rem % 60)
}

/// Date complète en infobulle (UTC explicite, le journal est une trace).
fn fmt_date(t: SystemTime) -> String {
    let secs = epoch_secs(t);
    let days = secs / 86400;
    let rem = secs % 86400;
    format!(
        "{} {:02}:{:02}:{:02} UTC",
        epoch_to_date(days),
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

fn epoch_secs(t: SystemTime) -> u64 {
    t.duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Décalage horaire local, lu une seule fois via `date` (aucune dépendance).
fn local_offset_secs() -> i64 {
    use std::sync::OnceLock;
    static OFFSET: OnceLock<i64> = OnceLock::new();
    *OFFSET.get_or_init(|| {
        std::process::Command::new("date")
            .arg("+%z")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| {
                let s = s.trim().to_string();
                if s.len() < 5 || !s.starts_with(['+', '-']) {
                    return None;
                }
                let sign = if s.starts_with('-') { -1 } else { 1 };
                let h: i64 = s[1..3].parse().ok()?;
                let m: i64 = s[3..5].parse().ok()?;
                Some(sign * (h * 3600 + m * 60))
            })
            .unwrap_or(0)
    })
}

fn epoch_to_date(days: u64) -> String {
    // Algorithme civil de Howard Hinnant.
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

/// Cellule de largeur fixe : la hauteur s'ajuste au contenu de la ligne.
fn cell(ui: &mut egui::Ui, w: f32, add: impl FnOnce(&mut egui::Ui)) {
    ui.allocate_ui_with_layout(
        egui::vec2(w, 20.0),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.set_width(w);
            add(ui);
        },
    );
}
