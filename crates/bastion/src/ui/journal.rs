//! Écran Journal : toutes les commandes exécutées, filtrables.

use crate::app::{GuiApp, JOURNAL_CAP, LogFilter};
use crate::components;
use crate::theme::Palette;
use egui::RichText;
use std::time::SystemTime;

pub fn render(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    components::screen_header(ui, p, t.nav_journal, t.journal_subtitle);
    ui.add_space(6.0);

    components::card_body(ui, p, |ui| {
        ui.horizontal(|ui| {
            for (f, label) in [
                (LogFilter::All, t.filter_all),
                (LogFilter::Errors, t.filter_errors),
                (LogFilter::Fw, t.filter_fw),
                (LogFilter::F2b, t.filter_f2b),
            ] {
                let sel = app.filter == f;
                let btn = egui::Button::new(
                    RichText::new(label).size(12.5).color(if sel {
                        egui::Color32::WHITE
                    } else {
                        p.text_secondary
                    }),
                )
                .fill(if sel { p.accent } else { p.card_hover });
                if ui.add(btn).clicked() {
                    app.filter = f;
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                if ui.add(egui::Button::new(RichText::new(t.clear_btn).size(12.5))).clicked() {
                    app.journal.clear();
                }
                if ui
                    .add(egui::Button::new(components::btn_icon_text(crate::icons::COPY, t.copy_btn)))
                    .on_hover_text(t.copy_tooltip)
                    .clicked()
                {
                    let all: Vec<String> = app
                        .journal
                        .iter()
                        .map(|e| {
                            format!(
                                "{} {} {} code={}",
                                fmt_time(e.at),
                                if e.ok { "OK  " } else { "ERR " },
                                e.command,
                                e.code
                            )
                        })
                        .collect();
                    ui.ctx().copy_text(all.join("\n"));
                    app.toasts.info(t.journal_copied_toast.to_string());
                }
            });
        });
        ui.add_space(4.0);

        if app.journal.is_empty() {
            components::empty_state(ui, p, t.journal_empty, t.journal_empty_hint);
            return;
        }
        ui.label(
            RichText::new(t.entries_count(app.journal.len(), JOURNAL_CAP))
                .size(11.0)
                .color(p.text_secondary),
        );

        egui::ScrollArea::both()
            .id_salt("journal-scroll")
            .max_height(560.0)
            .min_scrolled_height(560.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("journal-table")
                    .striped(true)
                    .num_columns(3)
                    .spacing(egui::vec2(14.0, 6.0))
                    .show(ui, |ui| {
                        for entry in app.journal.iter().rev() {
                            if app.filter == LogFilter::Errors && entry.ok {
                                continue;
                            }
                            if app.filter == LogFilter::Fw && !is_firewall_line(&entry.command) {
                                continue;
                            }
                            if app.filter == LogFilter::F2b && !is_f2b_line(&entry.command) {
                                continue;
                            }
                            ui.label(
                                RichText::new(fmt_time(entry.at))
                                    .size(11.5)
                                    .monospace()
                                    .color(p.text_secondary),
                            );
                            ui.label(
                                RichText::new(if entry.ok { "✔" } else { "✘" })
                                    .size(12.0)
                                    .color(if entry.ok { p.success } else { p.danger }),
                            );
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(components::ellipsize(&entry.command, 120)).monospace().size(12.0),
                                )
                                .on_hover_text(&entry.command);
                                if !entry.ok && !entry.detail.is_empty() {
                                    ui.label(
                                        RichText::new(components::ellipsize(&entry.detail, 160))
                                            .monospace()
                                            .size(11.0)
                                            .color(p.danger),
                                    )
                                    .on_hover_text(&entry.detail);
                                }
                            });
                            ui.end_row();
                        }
                    });
            });
    });
}

fn is_firewall_line(cmd: &str) -> bool {
    cmd.starts_with("ufw") || cmd.starts_with("journalctl -k")
}

fn is_f2b_line(cmd: &str) -> bool {
    cmd.starts_with("fail2ban-client")
        || cmd.starts_with("systemctl")
        || cmd.starts_with("journalctl -u fail2ban")
}

fn fmt_time(t: SystemTime) -> String {
    // Heure UTC explicite : le journal est une trace technique.
    let secs = t
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = secs / 86400;
    let rem = secs % 86400;
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    format!("{} {h:02}:{m:02}:{s:02} UTC", epoch_to_date(days))
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