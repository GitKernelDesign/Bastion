//! Mise en page globale : barre latérale, barre supérieure, barre d'état.

pub mod connect;
pub mod fail2ban;
pub mod firewall;
pub mod journal;
pub mod settings;

use crate::app::{ConnPhase, GuiApp, Screen};
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

// ─── Barre supérieure ────────────────────────────────────────────────────────
fn topbar(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    egui::Panel::top("topbar")
        .exact_size(52.0)
        .frame(egui::Frame::NONE.fill(p.bg).inner_margin(Margin{ left: 20, right: 20, top: 8, bottom: 8 }))
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                let (color, dot_icon, label) = match app.phase {
                    ConnPhase::Connected => (
                        if app.conn_desc.starts_with("SSH") { p.accent } else { p.success },
                        icons::GPP_GOOD,
                        app.conn_desc.clone(),
                    ),
                    ConnPhase::Connecting => (p.warning, icons::GPP_MAYBE, t.status_connecting.to_string()),
                    ConnPhase::Disconnected => (p.text_secondary, icons::GPP_BAD, t.status_disconnected.to_string()),
                };
                ui.label(icons::icon_sz(dot_icon, 18.0, color));
                ui.label(
                    RichText::new(label)
                        .size(13.0)
                        .family(egui::FontFamily::Name("Medium".into())),
                );

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let refreshing = app.is_refreshing();
                    let btn = egui::Button::new(icons::icon(icons::REFRESH).size(16.0))
                        .fill(if refreshing { p.card_hover } else { Color32::TRANSPARENT });
                    if ui
                        .add_enabled(app.phase == ConnPhase::Connected && !refreshing, btn)
                        .on_hover_text(t.tooltip_refresh)
                        .clicked()
                    {
                        app.refresh_all();
                    }

                    let mut auto = app.cfg.auto_refresh;
                    if ui
                        .add_enabled(
                            app.phase == ConnPhase::Connected,
                            egui::Checkbox::new(&mut auto, RichText::new(t.topbar_auto).size(12.5)),
                        )
                        .on_hover_text(t.tooltip_auto)
                        .changed()
                    {
                        app.cfg.auto_refresh = auto;
                        app.cfg.save();
                    }
                });
            });
        });
}

// ─── Barre latérale ──────────────────────────────────────────────────────────
fn sidebar(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    egui::Panel::left("nav")
        .exact_size(216.0)
        .frame(egui::Frame::NONE.fill(p.bg).inner_margin(Margin::same(12)))
        .show(ui, |ui| {
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.label(icons::icon_sz(icons::SHIELD, 26.0, p.accent));
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Bastion")
                            .size(17.0)
                            .family(egui::FontFamily::Name("SemiBold".into())),
                    );
                    ui.label(
                        RichText::new(t.app_tagline)
                            .size(11.0)
                            .color(p.text_secondary),
                    );
                });
            });
            ui.add_space(16.0);

            let entries = [
                (Screen::Connect, icons::LOGIN, t.nav_connect),
                (Screen::Firewall, icons::SECURITY, t.nav_firewall),
                (Screen::Fail2Ban, icons::BAN, t.nav_fail2ban),
                (Screen::Journal, icons::HISTORY, t.nav_journal),
                (Screen::Settings, icons::SETTINGS, t.nav_settings),
            ];
            for (screen, icon, label) in entries {
                nav_item(ui, p, screen, icon, label, &mut app.screen);
            }

            ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                let (col, txt) = match app.phase {
                    ConnPhase::Connected => (p.success, app.conn_desc.clone()),
                    ConnPhase::Connecting => (p.warning, t.status_connecting.to_string()),
                    ConnPhase::Disconnected => (p.text_secondary, t.status_disconnected.to_string()),
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
        RichText::new(format!("{}  {}", icon, label))
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

// ─── Barre d'état ────────────────────────────────────────────────────────────
fn statusbar(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    let pending = app.pending.len();
    egui::Panel::bottom("statusbar")
        .exact_size(26.0)
        .frame(egui::Frame::NONE.fill(p.bg).inner_margin(Margin{ left: 20, right: 20, top: 4, bottom: 4 }))
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                if pending > 0 {
                    ui.add(egui::Spinner::new().size(11.0));
                    ui.label(
                        RichText::new(t.ops_pending(pending))
                            .size(11.0)
                            .color(p.text_secondary),
                    );
                } else if !app.status_msg.is_empty() {
                    // Coupe proprement : le texte intégral est dans le toast
                    // et le journal.
                    let msg: String = app.status_msg.chars().take(110).collect();
                    let msg = if app.status_msg.chars().count() > 110 {
                        format!("{msg}…")
                    } else {
                        msg
                    };
                    ui.label(RichText::new(msg).size(11.0).color(p.text_secondary));
                } else {
                    ui.label(
                        RichText::new(t.status_ready)
                            .size(11.0)
                            .color(p.text_secondary),
                    );
                }
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    if let (Some(at), true) = (app.firewall.status_at, app.phase == ConnPhase::Connected) {
                        components::since_label(ui, p, t, at);
                    }
                    ui.label(
                        RichText::new(t.shortcut_hint)
                            .size(10.5)
                            .color(p.text_secondary),
                    );
                });
            });
        });
}

use crate::components;