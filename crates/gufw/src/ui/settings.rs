//! Écran Paramètres : thème, taille de texte, actualisation, réglages SSH.

use crate::app::GuiApp;
use crate::components;
use crate::config::ThemeKind;
use crate::theme::Palette;
use egui::RichText;

pub fn render(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    components::screen_header(
        ui,
        p,
        "Paramètres",
        "Apparence, actualisation et emplacement des fichiers de configuration.",
    );
    ui.add_space(6.0);

    components::card_body(ui, p, |ui| {
        ui.label(section("Apparence"));
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            components::hint(ui, p, "Thème");
            let mut theme = app.cfg.theme;
            components::segmented(
                ui,
                p,
                &[(ThemeKind::Dark, "Sombre"), (ThemeKind::Light, "Clair")],
                &mut theme,
            );
            if theme != app.cfg.theme {
                app.cfg.theme = theme;
                app.cfg.save();
                crate::theme::apply(ui.ctx(), app.cfg.theme, app.cfg.font_scale);
            }
        });

        ui.horizontal(|ui| {
            components::hint(ui, p, "Taille du texte");
            let mut scale = app.cfg.font_scale;
            components::segmented(
                ui,
                p,
                &[(0.9, "S"), (1.0, "M"), (1.15, "L")],
                &mut scale,
            );
            if scale != app.cfg.font_scale {
                app.cfg.font_scale = scale;
                app.cfg.save();
                crate::theme::apply(ui.ctx(), app.cfg.theme, app.cfg.font_scale);
            }
        });

        ui.horizontal(|ui| {
            let mut toasts = app.cfg.success_toasts;
            ui.checkbox(&mut toasts, "Notifications de succès (4 s)");
            if toasts != app.cfg.success_toasts {
                app.cfg.success_toasts = toasts;
                app.cfg.save();
            }
        });
    });

    ui.add_space(12.0);

    components::card_body(ui, p, |ui| {
        ui.label(section("Actualisation automatique"));
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let mut auto = app.cfg.auto_refresh;
            ui.checkbox(&mut auto, "Actualiser l'état périodiquement");
            if auto != app.cfg.auto_refresh {
                app.cfg.auto_refresh = auto;
                app.cfg.save();
            }
        });
        let mut secs = app.cfg.refresh_secs as f32;
        ui.horizontal(|ui| {
            components::hint(ui, p, "Intervalle");
            ui.add_enabled(
                app.cfg.auto_refresh,
                egui::Slider::new(&mut secs, 3.0..=60.0).suffix(" s"),
            );
            if secs as u64 != app.cfg.refresh_secs {
                app.cfg.refresh_secs = secs.max(3.0) as u64;
                app.cfg.save();
            }
        });
        components::hint(ui, p, "Une actualisation silencieuse n'interrompt jamais la saisie.");
    });

    ui.add_space(12.0);

    components::card_body(ui, p, |ui| {
        ui.label(section("Fichiers"));
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            components::hint(ui, p, "Configuration :");
            ui.monospace(
                crate::config::AppConfig::config_path()
                    .to_string_lossy()
                    .as_ref(),
            );
        });
        ui.horizontal(|ui| {
            components::hint(ui, p, "Profils SSH, thème, empreintes connues — aucun mot de passe.");
        });
    });

    ui.add_space(12.0);

    components::card_body(ui, p, |ui| {
        ui.label(section("À propos"));
        ui.add_space(6.0);
        components::hint(
            ui,
            p,
            &format!(
                "GUFW v{} — interface de gestion UFW & Fail2Ban (local et SSH).",
                env!("CARGO_PKG_VERSION")
            ),
        );
        components::hint(
            ui,
            p,
            "En local, un assistant root minimal est démarré via polkit une fois par session : liste blanche stricte (ufw, fail2ban-client, systemctl, journalctl — unités fail2ban/ufw uniquement), socket UNIX réservée à votre utilisateur, arrêt automatique après inactivité.",
        );
    });
}

fn section(s: &str) -> RichText {
    RichText::new(s)
        .size(13.0)
        .family(egui::FontFamily::Name("SemiBold".into()))
}
