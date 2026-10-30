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
        .frame(egui::Frame::NONE.fill(p.bg_alt).inner_margin(Margin{ left: 20, right: 20, top: 8, bottom: 8 }))
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
        .frame(egui::Frame::NONE.fill(p.bg_alt).inner_margin(Margin::same(12)))
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
            ui.add_space(14.0);
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 4.0);

            let entries = [
                (Screen::Connect, icons::LOGIN, t.nav_connect),
                (Screen::Firewall, icons::SECURITY, t.nav_firewall),
                (Screen::Fail2Ban, icons::BAN, t.nav_fail2ban),
                (Screen::Journal, icons::HISTORY, t.nav_journal),
                (Screen::Settings, icons::SETTINGS, t.nav_settings),
            ];
            let _ = nav_list(ui, p, &entries, &mut app.screen);

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

fn nav_item(ui: &mut egui::Ui, p: &Palette, screen: Screen, icon: &str, label: &str, current: &mut Screen) -> egui::Rect {
    let selected = *current == screen;
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 38.0), egui::Sense::click());
    resp.widget_info(|| {
        egui::WidgetInfo::selected(egui::Role::Button, true, selected, label)
    });
    let fill = if selected {
        crate::components::blend(p.accent, p.card, 0.15)
    } else if resp.hovered() {
        p.card_hover
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect(rect, egui::CornerRadius::same(9), fill, egui::Stroke::NONE, egui::StrokeKind::Inside);
    // Indicateur vertical de l'écran actif.
    if selected {
        let ind = egui::Rect::from_min_size(
            egui::pos2(rect.left() + 4.0, rect.center().y - 8.0),
            egui::vec2(3.0, 16.0),
        );
        ui.painter().rect_filled(ind, egui::CornerRadius::same(2), p.accent);
    }
    let fg = if selected { p.accent } else { p.text_secondary };
    let icon_g = ui.painter().layout_no_wrap(
        icon.to_string(),
        egui::FontId::new(17.0, egui::FontFamily::Name("Icon".into())),
        fg,
    );
    let txt_g = ui.painter().layout_no_wrap(
        label.to_string(),
        egui::FontId::new(14.5, egui::FontFamily::Name("Medium".into())),
        fg,
    );
    let y = rect.center().y;
    let mut x = rect.left() + 14.0;
    ui.painter().galley(egui::pos2(x, y - icon_g.size().y * 0.5), icon_g.clone(), fg);
    x += icon_g.size().x + 10.0;
    ui.painter().galley(egui::pos2(x, y - txt_g.size().y * 0.5), txt_g, fg);
    if resp.clicked() {
        *current = screen;
    }
    rect
}

// ─── Barre d'état ────────────────────────────────────────────────────────────
fn statusbar(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    let pending = app.pending.len();
    egui::Panel::bottom("statusbar")
        .exact_size(26.0)
        .frame(egui::Frame::NONE.fill(p.bg_alt).inner_margin(Margin{ left: 20, right: 20, top: 4, bottom: 4 }))
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
/// Liste de navigation verticale (extraite pour être testable).
fn nav_list(
    ui: &mut egui::Ui,
    p: &Palette,
    entries: &[(Screen, &str, &str)],
    current: &mut Screen,
) -> Vec<egui::Rect> {
    ui.spacing_mut().item_spacing = egui::vec2(8.0, 4.0);
    entries
        .iter()
        .map(|(screen, icon, label)| nav_item(ui, p, *screen, icon, label, current))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ThemeKind;

    /// Les items de navigation doivent s'empiler verticalement et rester
    /// dans les limites de l'écran, quelle que soit la largeur de fenêtre.
    #[test]
    fn nav_items_empiles_dans_l_ecran() {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx);
        let p = Palette::new(ThemeKind::Dark);
        let entries = [
            (Screen::Connect, icons::LOGIN, "Connexion"),
            (Screen::Firewall, icons::SECURITY, "Pare-feu"),
            (Screen::Fail2Ban, icons::BAN, "Fail2Ban"),
            (Screen::Journal, icons::HISTORY, "Journal"),
            (Screen::Settings, icons::SETTINGS, "Réglages"),
        ];
        for w in [1280.0, 900.0, 640.0] {
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, 720.0));
            let input = egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            };
            let mut current = Screen::Firewall;
            ctx.begin_pass(input);
            let rects = ctx.root_ui(|ui| {
                egui::Panel::left("nav")
                    .exact_size(216.0)
                    .frame(egui::Frame::NONE.fill(p.bg_alt).inner_margin(Margin::same(12)))
                    .show(ui, |ui| {
                        ui.add_space(6.0);
                        ui.add_space(14.0);
                        nav_list(ui, &p, &entries, &mut current)
                    })
                    .inner
            });
            let mut output = ctx.end_pass();
            output.textures_delta.clear();
            // Toutes les formes peintes restent dans l'écran.
            for cs in &output.shapes {
                let r = cs.shape.visual_bounding_rect();
                assert!(
                    r.left() >= -1.0
                        && r.right() <= screen.width() + 1.0
                        && r.top() >= -1.0
                        && r.bottom() <= screen.height() + 1.0,
                    "forme hors écran (largeur {w}) : {r:?}"
                );
            }
            // 5 items de 38 px empilés verticalement, sans chevauchement.
            assert_eq!(rects.len(), 5, "5 items attendus (largeur {w})");
            for r in &rects {
                assert!((r.height() - 38.0).abs() < 0.5, "hauteur item : {r:?}");
                assert!(r.left() >= -1.0 && r.right() <= screen.width() + 1.0, "item hors écran : {r:?}");
            }
            for pair in rects.windows(2) {
                assert!(pair[1].top() >= pair[0].bottom() - 0.5, "chevauchement : {rects:?}");
            }
        }
    }

    /// Une ScrollArea imbriquée dans le défilement de page doit conserver
    /// sa hauteur voulue (sinon elle s'effondre à min_scrolled_size = 64 px,
    /// voir egui#1097) même quand la hauteur disponible est épuisée.
    #[test]
    fn scroll_area_imbriquee_garde_sa_hauteur() {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 700.0));
        let input = egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        };
        ctx.begin_pass(input);
        let mut inner_h = 0.0f32;
        ctx.root_ui(|ui| {
            egui::ScrollArea::vertical()
                .id_salt("page")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // Contenu qui épuise toute la hauteur disponible.
                    for _ in 0..60 {
                        ui.label("ligne de remplissage");
                    }
                    let resp = egui::ScrollArea::vertical()
                        .id_salt("table")
                        .max_height(900.0)
                        .min_scrolled_height(900.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.label("contenu du tableau");
                        });
                    inner_h = resp.inner_rect.height();
                });
        });
        let mut output = ctx.end_pass();
        output.textures_delta.clear();
        assert!(
            inner_h >= 899.0,
            "hauteur de la zone imbriquée attendue ~900, vue {inner_h}"
        );
    }
}
