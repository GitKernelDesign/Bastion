//! Écran Paramètres : langue, thème, actualisation, fichiers, à propos.

use crate::app::GuiApp;
use crate::components;
use crate::config::ThemeKind;
use crate::i18n::Lang;
use crate::theme::Palette;
use egui::RichText;

pub fn render(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    components::screen_header(ui, p, t.nav_settings, t.settings_subtitle);
    ui.add_space(6.0);

    // ── Apparence ──
    components::card_body(ui, p, |ui| {
        ui.label(section(t.section_appearance));
        ui.add_space(6.0);

        // Langue.
        ui.horizontal(|ui| {
            ui.label(RichText::new(t.language_label).size(12.5).color(p.text_secondary));
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
        ui.horizontal(|ui| {
            ui.label(RichText::new(t.theme_label).size(12.5).color(p.text_secondary));
            let mut theme = app.cfg.theme;
            components::segmented(
                ui,
                p,
                &[(ThemeKind::Dark, t.theme_dark), (ThemeKind::Light, t.theme_light)],
                &mut theme,
            );
            if theme != app.cfg.theme {
                app.cfg.theme = theme;
                app.cfg.save();
                crate::theme::apply(ui.ctx(), app.cfg.theme, app.cfg.font_scale);
            }
        });

        // Taille du texte.
        ui.horizontal(|ui| {
            ui.label(RichText::new(t.text_size_label).size(12.5).color(p.text_secondary));
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
    ui.add_space(12.0);

    // ── Actualisation ──
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
        ui.horizontal(|ui| {
            ui.label(RichText::new(t.interval_label).size(12.5).color(p.text_secondary));
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
    ui.add_space(12.0);

    // ── Fichiers ──
    components::card_body(ui, p, |ui| {
        ui.label(section(t.section_files));
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(t.config_path_label).size(12.5).color(p.text_secondary));
            ui.monospace(
                crate::config::AppConfig::config_path()
                    .to_string_lossy()
                    .as_ref(),
            );
        });
        components::hint(ui, p, t.files_hint);
    });
    ui.add_space(12.0);

    // ── À propos ──
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