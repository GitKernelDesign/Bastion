//! Écran Réglages : apparence, actualisation, données et sécurité, à propos.
//! Chaque paramètre est un contrôle direct, sans bouton « Enregistrer » :
//! tout est persisté immédiatement.

use crate::app::GuiApp;
use crate::components;
use crate::config::ThemeKind;
use crate::i18n::Lang;
use crate::theme::{space, text, Palette};
use egui::{Align, CornerRadius, Frame, Layout, Margin, RichText};

pub fn render(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    components::tile_grid(ui, p, 340.0, 4, |ui, p, i| match i {
        0 => appearance_card(app, ui, p),
        1 => refresh_card(app, ui, p),
        2 => data_card(app, ui, p),
        _ => about_card(app, ui, p),
    });
}

// ─── Apparence ───────────────────────────────────────────────────────────────
fn appearance_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    components::card(ui, p, t.section_appearance, |_| {}, |ui| {
        // Langue : nom natif, sélectionnable au clavier.
        components::labeled_control(ui, p, t.language_label, 420.0, |ui| {
            let current = app.lang;
            egui::ComboBox::from_id_salt("lang")
                .selected_text(current.native())
                .width((ui.available_width() - 8.0).clamp(140.0, 240.0))
                .show_ui(ui, |ui| {
                    for lang in Lang::all() {
                        if ui
                            .selectable_label(lang == current, lang.native())
                            .clicked()
                            && lang != current
                        {
                            app.set_language(ui.ctx(), lang);
                        }
                    }
                });
        });

        // Thème.
        components::labeled_control(ui, p, t.theme_label, 420.0, |ui| {
            let mut theme = app.cfg.theme;
            components::segmented(
                ui,
                p,
                &[
                    (ThemeKind::System, t.theme_system),
                    (ThemeKind::Light, t.theme_light),
                    (ThemeKind::Dark, t.theme_dark),
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

        // Taille du texte : A−, A, A+ (compréhensible dans les 8 langues).
        components::labeled_control(ui, p, t.text_size_label, 420.0, |ui| {
            let mut scale = app.cfg.font_scale;
            components::segmented(
                ui,
                p,
                &[(0.9, "A−"), (1.0, "A"), (1.15, "A+")],
                &mut scale,
            );
            if scale != app.cfg.font_scale {
                app.cfg.font_scale = scale;
                app.cfg.save();
                crate::theme::apply(ui.ctx(), app.cfg.theme, app.cfg.font_scale);
            }
        });

        ui.add_space(space::M);
        components::hairline(ui, p);
        ui.add_space(space::M);
        let mut toasts = app.cfg.success_toasts;
        if components::switch_row(
            ui,
            p,
            toasts,
            true,
            t.success_toasts_cb,
            "",
        ) != toasts
        {
            toasts = !toasts;
            app.cfg.success_toasts = toasts;
            app.cfg.save();
        }
    });
}

// ─── Actualisation ───────────────────────────────────────────────────────────
fn refresh_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    components::card(ui, p, t.section_refresh, |_| {}, |ui| {
        let mut auto = app.cfg.auto_refresh;
        if components::switch_row(ui, p, auto, true, t.auto_refresh_cb, t.refresh_hint) != auto {
            auto = !auto;
            app.cfg.auto_refresh = auto;
            app.cfg.save();
        }
        ui.add_space(space::M);
        components::labeled_control(ui, p, t.interval_label, 320.0, |ui| {
            // Curseur large : la valeur chiffrée reste lisible à côté.
            ui.spacing_mut().slider_width = (ui.available_width() - 92.0).clamp(120.0, 320.0);
            let mut secs = app.cfg.refresh_secs as f32;
            // Toujours modifiable : l'intervalle sert dès que l'actualisation
            // est réactivée, et un curseur grisé serait plus difficile à viser.
            if ui
                .add(egui::Slider::new(&mut secs, 3.0..=60.0).suffix(" s"))
                .changed()
            {
                app.cfg.refresh_secs = secs.max(3.0) as u64;
                app.cfg.save();
            }
        });
        ui.add_space(space::M);
        components::hairline(ui, p);
        ui.add_space(space::M);
        ui.label(
            RichText::new(t.settings_shortcuts_section)
                .size(text::MICRO)
                .color(p.text_secondary),
        );
        ui.add_space(2.0);
        components::hint(ui, p, t.shortcut_hint);
    });
}

// ─── Données et sécurité ─────────────────────────────────────────────────────
fn data_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    let folder = crate::config::AppConfig::config_dir();
    let path = folder.to_string_lossy().to_string();
    let mut open_folder = false;
    let mut forget = false;

    components::card(ui, p, t.settings_data_section, |_| {}, |ui| {
        ui.label(
            RichText::new(t.section_files)
                .size(text::MICRO)
                .color(p.text_secondary),
        );
        ui.add_space(2.0);
        Frame::NONE
            .fill(p.field)
            .stroke(egui::Stroke::new(1.0, p.border))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::symmetric(10, 7))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    RichText::new(components::ellipsize(&path, 46))
                        .monospace()
                        .size(text::MONO),
                )
                .on_hover_text(&path);
            });
        ui.add_space(space::S);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 6.0);
            if components::styled_button(
                ui,
                p,
                &format!("{}  {}", crate::icons::FOLDER_OPEN, t.settings_open_folder),
                components::BtnKind::Soft,
            )
            .clicked()
            {
                open_folder = true;
            }
            if components::styled_button(
                ui,
                p,
                &format!("{}  {}", crate::icons::DELETE, t.settings_forget_hosts),
                components::BtnKind::Quiet,
            )
            .clicked()
            {
                forget = true;
            }
        });
        ui.add_space(space::S);
        let hosts = app.cfg.known_hosts.len();
        components::hint(ui, p, &t.hosts_count(hosts));
        ui.add_space(space::XS);
        components::hint(ui, p, t.files_hint);
    });

    if open_folder {
        let _ = std::process::Command::new("xdg-open").arg(&folder).spawn();
    }
    if forget {
        app.cfg.known_hosts.clear();
        app.cfg.save();
        app.toasts.info(t.settings_hosts_forgotten.to_string());
    }
}

// ─── À propos ────────────────────────────────────────────────────────────────
fn about_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    components::card(ui, p, t.section_about, |_| {}, |ui| {
        ui.horizontal(|ui| {
            ui.add(egui::Image::new(&app.logo).max_size(egui::vec2(36.0, 36.0)));
            ui.add_space(2.0);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Bastion")
                        .size(15.0)
                        .family(egui::FontFamily::Name("SemiBold".into())),
                );
                ui.label(
                    RichText::new(concat!("v", env!("CARGO_PKG_VERSION")))
                        .size(text::MICRO)
                        .color(p.text_secondary),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                components::tag(ui, p, t.settings_licence, p.text_secondary);
            });
        });
        ui.add_space(space::M);
        components::hint(ui, p, &t.about_line(env!("CARGO_PKG_VERSION")));
        ui.add_space(space::S);
        components::hint(ui, p, t.about_helper);
    });
}
