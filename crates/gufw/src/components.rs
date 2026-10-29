//! Composants d'interface : cartes, interrupteur, sélecteur segmenté,
//! badges, boutons sémantiques. Tout ce qui donne à l'application son
//! langage visuel sobre et cohérent (voir docs/ERGONOMIE.md).

use crate::icons;
use crate::theme::Palette;
use egui::{Align, Button, Color32, CornerRadius, Frame, Layout, Margin, RichText, Sense, Ui};

/// Mélange une couleur vers un fond (k = part de la couleur).
pub fn blend(c: Color32, bg: Color32, k: f32) -> Color32 {
    let mix = |a: u8, b: u8| -> u8 { ((a as f32) * (1.0 - k) + (b as f32) * k) as u8 };
    Color32::from_rgb(mix(bg.r(), c.r()), mix(bg.g(), c.g()), mix(bg.b(), c.b()))
}

/// Carte avec en-tête (titre majuscules espacées + actions à droite).
pub fn card<R>(
    ui: &mut Ui,
    p: &Palette,
    title: &str,
    actions: impl FnOnce(&mut Ui),
    body: impl FnOnce(&mut Ui) -> R,
) -> R {
    Frame::NONE
        .fill(p.card)
        .stroke(egui::Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::same(16))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width() - 2.0);
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(title.to_uppercase())
                        .size(11.5)
                        .weak()
                        .family(egui::FontFamily::Name("Medium".into())),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.style_mut().spacing.item_spacing = egui::vec2(6.0, 6.0);
                    actions(ui);
                });
            });
            ui.add_space(6.0);
            body(ui)
        })
        .inner
}

/// Carte sans en-tête.
pub fn card_body<R>(ui: &mut Ui, p: &Palette, body: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::NONE
        .fill(p.card)
        .stroke(egui::Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::same(16))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width() - 2.0);
            body(ui)
        })
        .inner
}

/// Interrupteur à bascule (52×28 px, animé).
pub fn switch(ui: &mut Ui, p: &Palette, on: bool, accent: Option<Color32>) -> bool {
    let id = ui.scope_id().with("switch");
    let animated = ui.ctx().animate_bool(id, on);
    let size = egui::vec2(52.0, 28.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let track = CornerRadius::same(14);
    let bg = if on {
        accent.unwrap_or(p.success)
    } else {
        p.card_hover
    };
    ui.painter().rect_filled(rect, track, bg);
    // Position du bouton : 2 px de marge.
    let travel = rect.width() - 28.0;
    let x = rect.left() + 2.0 + animated * travel;
    let knob_rect = egui::Rect::from_min_size(
        egui::pos2(x, rect.center().y - 11.0),
        egui::vec2(22.0, 22.0),
    );
    ui.painter().circle_filled(knob_rect.center(), 11.0, Color32::WHITE);
    if resp.clicked() {
        !on
    } else {
        on
    }
}

/// Sélecteur segmenté. Retourne vrai si la valeur a changé.
pub fn segmented<T: PartialEq + Copy>(
    ui: &mut Ui,
    p: &Palette,
    variants: &[(T, &str)],
    value: &mut T,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        for (i, (v, label)) in variants.iter().enumerate() {
            let selected = *value == *v;
            let bg = if selected { p.accent } else { p.card_hover };
            let fg = if selected { Color32::WHITE } else { p.text };
            let btn = Button::new(
                RichText::new(*label)
                    .color(fg)
                    .family(egui::FontFamily::Name("Medium".into())),
            )
            .fill(bg)
            .corner_radius(8);
            if ui.add(btn).clicked() && *value != *v {
                *value = *v;
                changed = true;
            }
            if i < variants.len() - 1 {
                ui.add_space(2.0);
            }
        }
    });
    changed
}

/// Badge coloré (état, action…).
pub fn badge(ui: &mut Ui, _p: &Palette, text: &str, fg: Color32, bg: Color32) {
    Frame::NONE
        .fill(bg)
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin{ left: 8, right: 8, top: 3, bottom: 3 })
        .show(ui, |ui| {
            ui.label(
                RichText::new(text)
                    .size(11.5)
                    .color(fg)
                    .family(egui::FontFamily::Name("Medium".into())),
            );
        });
}

/// Badge d'action sémantique (Autoriser / Bloquer / Rejeter / Limiter).
pub fn action_badge(ui: &mut Ui, p: &Palette, text: &str, color: Color32) {
    let bg = blend(color, ui.style().visuals.panel_fill, 0.18);
    badge(ui, p, text, color, bg);
}

/// Point d'état + libellé.
pub fn status_line(ui: &mut Ui, p: &Palette, color: Color32, label: &str, sub: Option<&str>) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), Sense::hover());
        ui.painter().circle_filled(rect.center(), 5.0, color);
        ui.label(
            RichText::new(label)
                .size(16.0)
                .family(egui::FontFamily::Name("SemiBold".into())),
        );
        if let Some(s) = sub {
            ui.label(RichText::new(s).size(12.5).color(p.text_secondary));
        }
    });
}

/// Bouton principal (accent plein).
pub fn primary_button(ui: &mut Ui, p: &Palette, label: &str) -> egui::Response {
    ui.add(
        Button::new(
            RichText::new(label)
                .color(Color32::WHITE)
                .family(egui::FontFamily::Name("Medium".into())),
        )
        .fill(p.accent),
    )
}

/// Petit bouton principal avec icône (« + Ajouter »).
pub fn primary_button_small(ui: &mut Ui, p: &Palette, icon: &str, label: &str) -> egui::Response {
    ui.add(
        Button::new(
            RichText::new(format!("{}  {}", icon, label))
                .color(Color32::WHITE)
                .size(13.5)
                .family(egui::FontFamily::Name("Icon".into())),
        )
        .fill(p.accent),
    )
}

/// Bouton destructif (danger plein).
pub fn danger_button(ui: &mut Ui, p: &Palette, label: &str) -> egui::Response {
    ui.add(
        Button::new(
            RichText::new(label)
                .color(Color32::WHITE)
                .family(egui::FontFamily::Name("Medium".into())),
        )
        .fill(p.danger),
    )
}

/// Bouton icône discret, libellé en infobulle.
pub fn icon_button(ui: &mut Ui, _p: &Palette, icon: &str, tooltip: &str) -> egui::Response {
    let b = Button::new(icons::icon(icon).size(17.0))
        .fill(Color32::TRANSPARENT)
        .corner_radius(6);
    let resp = ui.add(b);
    resp.on_hover_text(tooltip)
}

/// Texte riche combinant icône et libellé (pour boutons).
pub fn btn_icon_text(icon: &str, label: &str) -> RichText {
    RichText::new(format!("{}  {}", icon, label))
        .family(egui::FontFamily::Name("Icon".into()))
}

/// Libellé de champ + saisie monoligne.
pub fn field(ui: &mut Ui, p: &Palette, label: &str, value: &mut String, hint: &str) {
    ui.vertical(|ui| {
        ui.label(
            RichText::new(label)
                .size(12.5)
                .color(p.text_secondary)
                .family(egui::FontFamily::Name("Medium".into())),
        );
        let edit = egui::TextEdit::singleline(value).desired_width(ui.available_width());
        let resp = ui.add(edit);
        if !hint.is_empty() {
            resp.on_hover_text(hint);
        }
    });
}

/// Champ mot de passe.
pub fn password_field(ui: &mut Ui, p: &Palette, label: &str, value: &mut String) {
    ui.vertical(|ui| {
        ui.label(
            RichText::new(label)
                .size(12.5)
                .color(p.text_secondary)
                .family(egui::FontFamily::Name("Medium".into())),
        );
        ui.add(
            egui::TextEdit::singleline(value)
                .password(true)
                .desired_width(ui.available_width()),
        );
    });
}

/// Aide contextuelle (texte secondaire, 12,5 px).
pub fn hint(ui: &mut Ui, p: &Palette, text: &str) {
    ui.label(RichText::new(text).size(12.5).color(p.text_secondary));
}

/// État vide (aucune donnée) avec message et suggestion.
pub fn empty_state(ui: &mut Ui, p: &Palette, message: &str, sub: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(14.0);
        ui.label(icons::icon_sz(icons::STORAGE, 34.0, p.text_secondary));
        ui.add_space(4.0);
        ui.label(
            RichText::new(message)
                .size(14.5)
                .family(egui::FontFamily::Name("Medium".into())),
        );
        ui.label(RichText::new(sub).size(12.5).color(p.text_secondary));
        ui.add_space(10.0);
    });
}

/// En-tête d'écran (titre 20 px + sous-titre).
pub fn screen_header(ui: &mut Ui, p: &Palette, title: &str, sub: &str) {
    ui.vertical(|ui| {
        ui.label(
            RichText::new(title)
                .size(20.0)
                .family(egui::FontFamily::Name("SemiBold".into())),
        );
        ui.label(RichText::new(sub).size(12.5).color(p.text_secondary));
        ui.add_space(8.0);
    });
}

/// Spinner de chargement discret + libellé.
pub fn loading(ui: &mut Ui, p: &Palette, label: &str) {
    ui.horizontal(|ui| {
        ui.add(egui::Spinner::new().size(14.0));
        ui.label(RichText::new(label).size(12.5).color(p.text_secondary));
    });
}

/// Affiche « il y a Ns » depuis un Instant.
pub fn since_label(ui: &mut Ui, p: &Palette, at: std::time::Instant) {
    let secs = at.elapsed().as_secs();
    let txt = match secs {
        0 => "à l'instant".to_string(),
        s if s < 60 => format!("il y a {s} s"),
        m if m < 3600 => format!("il y a {} min", m / 60),
        h => format!("il y a {} h", h / 3600),
    };
    ui.label(RichText::new(txt).size(11.5).color(p.text_secondary));
}

/// Fond de modale (frame utilisée par crate::modal).
pub fn modal_frame(p: &Palette) -> egui::Frame {
    Frame::NONE
        .fill(p.card)
        .stroke(egui::Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::same(18))
}

/// Titre de modale (rendu dans le contenu de la modale).
pub fn modal_title(ui: &mut Ui, _p: &Palette, title: &str) {
    ui.label(
        RichText::new(title)
            .size(16.0)
            .family(egui::FontFamily::Name("SemiBold".into())),
    );
    ui.add_space(8.0);
}

