//! Écran Journal : toutes les commandes exécutées, horodatées, filtrables.

use crate::app::{GuiApp, JOURNAL_CAP, LogFilter};
use crate::components;
use crate::theme::Palette;
use egui::RichText;
use std::time::SystemTime;

pub fn render(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    components::screen_header(
        ui,
        p,
        "Journal",
        "Chaque commande passée au système, avec son résultat. Rien ne s'exécute en silence.",
    );
    ui.add_space(6.0);

    components::card_body(ui, p, |ui| {
        // Filtres
        ui.horizontal(|ui| {
            let current = app.filter;
            for (f, label) in [
                (LogFilter::All, "Tout"),
                (LogFilter::Errors, "Erreurs"),
                (LogFilter::Ufw, "Pare-feu"),
                (LogFilter::Fail2ban, "Fail2Ban"),
            ] {
                let sel = current == f;
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
                if ui
                    .add(egui::Button::new(RichText::new("Tout effacer").size(12.5)))
                    .clicked()
                {
                    app.journal.clear();
                }
                if ui
                    .add(egui::Button::new(components::btn_icon_text(crate::icons::COPY, "Copier")))
                    .on_hover_text("Copie le journal complet dans le presse-papier")
                    .clicked()
                {
                    let all: Vec<String> = app
                        .journal
                        .iter()
                        .map(|e| {
                            format!(
                                "{} {} {} → code {}",
                                fmt_time(e.at),
                                if e.ok { "OK " } else { "ERR" },
                                e.command,
                                e.code
                            )
                        })
                        .collect();
                    ui.ctx().copy_text(all.join("\n"));
                    app.toasts.info("Journal copié");
                }
            });
        });
        ui.add_space(4.0);

        if app.journal.is_empty() {
            components::empty_state(
                ui,
                p,
                "Journal vide",
                "Les commandes exécutées par l'application apparaîtront ici.",
            );
            return;
        }

        ui.label(
            RichText::new(format!("{} entrées (plafonné à {JOURNAL_CAP})", app.journal.len()))
                .size(11.0)
                .color(p.text_secondary),
        );

        egui::ScrollArea::vertical()
            .max_height(560.0)
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
                            if app.filter == LogFilter::Ufw
                                && !entry.command.starts_with("ufw")
                                && !entry.command.starts_with("journalctl -k")
                            {
                                continue;
                            }
                            if app.filter == LogFilter::Fail2ban
                                && !entry.command.starts_with("fail2ban-client")
                                && !entry.command.starts_with("systemctl")
                                && !entry.command.starts_with("journalctl -u fail2ban")
                            {
                                continue;
                            }

                            ui.label(
                                RichText::new(fmt_time(entry.at))
                                    .size(11.5)
                                    .color(p.text_secondary)
                                    .monospace(),
                            );
                            ui.label(
                                RichText::new(if entry.ok { "✔" } else { "✘" })
                                    .size(12.0)
                                    .color(if entry.ok { p.success } else { p.danger }),
                            );
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&entry.command).monospace().size(12.0));
                                if !entry.ok && !entry.detail.is_empty() {
                                    ui.label(
                                        RichText::new(&entry.detail)
                                            .monospace()
                                            .size(11.0)
                                            .color(p.danger),
                                    );
                                }
                            });
                            ui.end_row();
                        }
                    });
            });
    });
}

fn fmt_time(t: SystemTime) -> String {
    // Heure locale approximée par l'heure UTC : suffisant pour un journal
    // d'audit ; l'affichage relatif est dans la barre d'état.
    let secs = t
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let day = secs / 86400;
    let rem = secs % 86400;
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let date = epoch_to_date(day);
    format!("{date} {h:02}:{m:02}:{s:02} UTC")
}

fn epoch_to_date(days: u64) -> String {
    // Algorithme civil de conversion jours→date (Howard Hinnant).
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
