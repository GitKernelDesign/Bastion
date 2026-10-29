//! Mise en page globale : barre latérale, barre supérieure, barre d'état,
//! et répartition vers les écrans.

pub mod connect;
pub mod fail2ban;
pub mod firewall;
pub mod journal;
pub mod settings;

use crate::app::{ConnPhase, GuiApp, Screen};
use crate::components;
use crate::icons;
use crate::theme::Palette;
use egui::{Align, Color32, Layout, Margin, RichText};

/// Rendu complet d'une frame.
pub fn render(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    topbar(app, ui, p);
    statusbar(app, ui, p);
    sidebar(app, ui, p);

    egui::CentralPanel::default_margins()
        .frame(
            egui::Frame::NONE
                .fill(p.bg)
                .inner_margin(Margin::same(20)),
        )
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| match app.screen {
                    Screen::Connect => connect::render(app, ui, p),
                    Screen::Firewall => firewall::render(app, ui, p),
                    Screen::Fail2Ban => fail2ban::render(app, ui, p),
                    Screen::Journal => journal::render(app, ui, p),
                    Screen::Settings => settings::render(app, ui, p),
                });
        });
}

// --------------------------------------------------------------------- barre supérieure

fn topbar(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    egui::Panel::top("topbar")
        .exact_size(52.0)
        .frame(
            egui::Frame::NONE
                .fill(p.bg)
                .inner_margin(Margin{ left: 20, right: 20, top: 8, bottom: 8 }),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                // Indicateur de connexion (question la plus fréquente :
                // « sur quelle machine suis-je en train d'agir ? »)
                let (color, dot_icon, label) = match app.phase {
                    ConnPhase::Connected => (
                        if app.conn_desc.starts_with("SSH") { p.accent } else { p.success },
                        icons::GPP_GOOD,
                        app.conn_desc.clone(),
                    ),
                    ConnPhase::Connecting => (p.warning, icons::GPP_MAYBE, "Connexion…".into()),
                    ConnPhase::Disconnected => (p.text_secondary, icons::GPP_BAD, "Non connecté".into()),
                };
                ui.label(icons::icon_sz(dot_icon, 18.0, color));
                ui.label(
                    RichText::new(label)
                        .size(13.0)
                        .family(egui::FontFamily::Name("Medium".into())),
                );

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    // Actualisation manuelle
                    let refreshing = app.is_refreshing();
                    let btn = egui::Button::new(icons::icon(icons::REFRESH).size(16.0))
                        .fill(if refreshing { p.card_hover } else { Color32::TRANSPARENT });
                    if ui
                        .add_enabled(app.phase == ConnPhase::Connected && !refreshing, btn)
                        .on_hover_text("Actualiser (Ctrl+R)")
                        .clicked()
                    {
                        app.refresh_all();
                    }

                    // Auto-actualisation
                    let mut auto = app.cfg.auto_refresh;
                    ui.add_enabled(
                        app.phase == ConnPhase::Connected,
                        egui::Checkbox::new(&mut auto, RichText::new("Auto").size(12.5)),
                    )
                    .on_hover_text("Actualise l'état périodiquement");
                    if auto != app.cfg.auto_refresh {
                        app.cfg.auto_refresh = auto;
                        app.cfg.save();
                    }
                });
            });
        });
}

// --------------------------------------------------------------------- navigation

fn sidebar(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    egui::Panel::left("nav")
        .exact_size(216.0)
        .frame(
            egui::Frame::NONE
                .fill(p.bg)
                .inner_margin(Margin::same(12)),
        )
        .show(ui, |ui| {
            ui.add_space(6.0);

            // Marque.
            ui.horizontal(|ui| {
                ui.label(icons::icon_sz(icons::SHIELD, 26.0, p.accent));
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("GUFW")
                            .size(17.0)
                            .family(egui::FontFamily::Name("SemiBold".into())),
                    );
                    ui.label(RichText::new("UFW & Fail2Ban").size(11.0).color(p.text_secondary));
                });
            });
            ui.add_space(16.0);

            let entries: [(Screen, &str, &str); 5] = [
                (Screen::Connect, icons::LOGIN, "Connexion"),
                (Screen::Firewall, icons::SECURITY, "Pare-feu"),
                (Screen::Fail2Ban, icons::BAN, "Fail2Ban"),
                (Screen::Journal, icons::HISTORY, "Journal"),
                (Screen::Settings, icons::SETTINGS, "Paramètres"),
            ];
            for (screen, icon, label) in entries {
                nav_item(ui, p, screen, icon, label, &mut app.screen);
            }

            // En bas : rappel du mode de connexion.
            ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                let (col, txt) = match app.phase {
                    ConnPhase::Connected => (p.success, app.conn_desc.as_str()),
                    ConnPhase::Connecting => (p.warning, "Connexion…"),
                    ConnPhase::Disconnected => (p.text_secondary, "Non connecté"),
                };
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 4.0, col);
                    ui.label(RichText::new(txt).size(11.5).color(p.text_secondary));
                });
                ui.label(
                    RichText::new(concat!("v", env!("CARGO_PKG_VERSION")))
                        .size(10.5)
                        .color(p.text_secondary),
                );
            });
        });
}

fn nav_item(ui: &mut egui::Ui, p: &Palette, screen: Screen, icon: &str, label: &str, current: &mut Screen) {
    let selected = *current == screen;
    let bg = if selected { p.card_hover } else { Color32::TRANSPARENT };
    let fg = if selected { p.text } else { p.text_secondary };
    let btn = egui::Button::new(
        egui::RichText::new(format!("{}  {}", icon, label))
            .family(egui::FontFamily::Name("Icon".into()))
            .color(fg)
            .size(15.0),
    )
    .fill(bg)
    .corner_radius(egui::CornerRadius::same(8))
    .min_size(egui::vec2(ui.available_width(), 38.0));
    if ui.add(btn).clicked() {
        *current = screen;
    }
}

// --------------------------------------------------------------------- barre d'état

fn statusbar(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let msg = app.status_msg.clone();
    let pending = app.pending.len();
    egui::Panel::bottom("statusbar")
        .exact_size(26.0)
        .frame(
            egui::Frame::NONE
                .fill(p.bg)
                .inner_margin(Margin{ left: 20, right: 20, top: 4, bottom: 4 }),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                if pending > 0 {
                    ui.add(egui::Spinner::new().size(11.0));
                    ui.label(RichText::new(format!("{pending} opération(s) en cours")).size(11.0).color(p.text_secondary));
                } else if !msg.is_empty() {
                    ui.label(RichText::new(msg).size(11.0).color(p.text_secondary));
                } else {
                    ui.label(RichText::new("Prêt").size(11.0).color(p.text_secondary));
                }
                // Fraîcheur des données.
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    if let Some(at) = app.firewall.status_at {
                        if app.phase == ConnPhase::Connected {
                            components::since_label(ui, p, at);
                        }
                    }
                    ui.label(
                        RichText::new("Ctrl+1…5 : écrans · Ctrl+R : actualiser · Ctrl+N : règle")
                            .size(10.5)
                            .color(p.text_secondary),
                    );
                });
            });
        });
}
