//! Écran Paramètres : langue, thème, actualisation, fichiers, à propos.
//! Cartes en grille responsive (2×2 en fenêtre large, empilées sinon).

use crate::app::GuiApp;
use crate::components;
use crate::config::ThemeKind;
use crate::i18n::Lang;
use crate::theme::Palette;
use egui::RichText;

pub fn render(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    components::screen_header(ui, p, app.tr.nav_settings, app.tr.settings_subtitle);
    ui.add_space(6.0);

    // Grille responsive : jusqu'à 2×2 tuiles en fenêtre large, une seule
    // colonne en fenêtre étroite (mobile-first).
    components::tile_grid(ui, p, 400.0, 4, |ui, p, i| match i {
        0 => appearance_card(app, ui, p),
        1 => refresh_card(app, ui, p),
        2 => files_card(app, ui, p),
        _ => about_card(app, ui, p),
    });
}

/// ── Apparence ──
fn appearance_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    components::card_body(ui, p, |ui| {
        ui.label(section(t.section_appearance));
        ui.add_space(6.0);

        // Langue.
        components::labeled_control(ui, p, t.language_label, 380.0, |ui| {
            let mut lang = app.lang;
            components::segmented(
                ui,
                p,
                &[
                    (Lang::Fr, Lang::Fr.code()),
                    (Lang::En, Lang::En.code()),
                    (Lang::Es, Lang::Es.code()),
                    (Lang::De, Lang::De.code()),
                ],
                &mut lang,
            );
            if lang != app.lang {
                app.set_language(ui.ctx(), lang);
            }
        });

        // Thème.
        components::labeled_control(ui, p, t.theme_label, 380.0, |ui| {
            let mut theme = app.cfg.theme;
            components::segmented(
                ui,
                p,
                &[
                    (ThemeKind::System, t.theme_system),
                    (ThemeKind::Dark, t.theme_dark),
                    (ThemeKind::Light, t.theme_light),
                ],
                &mut theme,
            );
            if theme != app.cfg.theme {
                app.cfg.theme = theme;
                app.cfg.save();
                app.resolved_theme = crate::theme::resolve(theme);
                crate::theme::apply(ui.ctx(), app.cfg.theme, app.cfg.font_scale);
            }
        });

        // Taille du texte.
        components::labeled_control(ui, p, t.text_size_label, 380.0, |ui| {
            let mut scale = app.cfg.font_scale;
            components::segmented(ui, p, &[(0.9, "S"), (1.0, "M"), (1.15, "L")], &mut scale);
            if scale != app.cfg.font_scale {
                app.cfg.font_scale = scale;
                app.cfg.save();
                crate::theme::apply(ui.ctx(), app.cfg.theme, app.cfg.font_scale);
            }
        });

        ui.horizontal(|ui| {
            let mut toasts = app.cfg.success_toasts;
            if ui.checkbox(&mut toasts, t.success_toasts_cb).changed() {
                app.cfg.success_toasts = toasts;
                app.cfg.save();
            }
        });
    });
}

/// ── Actualisation ──
fn refresh_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    components::card_body(ui, p, |ui| {
        ui.label(section(t.section_refresh));
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let mut auto = app.cfg.auto_refresh;
            if ui.checkbox(&mut auto, t.auto_refresh_cb).changed() {
                app.cfg.auto_refresh = auto;
                app.cfg.save();
            }
        });
        components::labeled_control(ui, p, t.interval_label, 360.0, |ui| {
            let mut secs = app.cfg.refresh_secs as f32;
            if ui
                .add_enabled(app.cfg.auto_refresh, egui::Slider::new(&mut secs, 3.0..=60.0).suffix(" s"))
                .changed()
            {
                app.cfg.refresh_secs = secs.max(3.0) as u64;
                app.cfg.save();
            }
        });
        components::hint(ui, p, t.refresh_hint);
    });
}

/// ── Fichiers ──
fn files_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    components::card_body(ui, p, |ui| {
        ui.label(section(t.section_files));
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 6.0);
            ui.label(RichText::new(t.config_path_label).size(12.5).color(p.text_secondary));
            let path = crate::config::AppConfig::config_path()
                .to_string_lossy()
                .to_string();
            ui.label(RichText::new(components::ellipsize(&path, 58)).monospace())
                .on_hover_text(&path);
        });
        components::hint(ui, p, t.files_hint);
    });
}

/// ── À propos ──
fn about_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    components::card_body(ui, p, |ui| {
        ui.label(section(t.section_about));
        ui.add_space(6.0);
        components::hint(ui, p, &t.about_line(env!("CARGO_PKG_VERSION")));
        components::hint(ui, p, t.about_helper);
    });
}

fn section(s: &str) -> RichText {
    RichText::new(s)
        .size(13.0)
        .family(egui::FontFamily::Name("SemiBold".into()))
}