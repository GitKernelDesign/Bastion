//! Composants d'interface : cartes, interrupteur, sélecteur segmenté,
//! badges, boutons sémantiques. Tout ce qui donne à l'application son
//! langage visuel sobre et cohérent (voir docs/ERGONOMIE.md).

use crate::icons;
use crate::theme::Palette;
use egui::{
    Align, Color32, CornerRadius, FontId, Frame, Layout, Margin, RichText, Sense, Stroke,
    StrokeKind, Ui,
};

/// Mélange une couleur vers un fond (k = part de la couleur).
pub fn blend(c: Color32, bg: Color32, k: f32) -> Color32 {
    let mix = |a: u8, b: u8| -> u8 { ((a as f32) * (1.0 - k) + (b as f32) * k) as u8 };
    Color32::from_rgb(mix(bg.r(), c.r()), mix(bg.g(), c.g()), mix(bg.b(), c.b()))
}

const WHITE: Color32 = Color32::from_rgb(255, 255, 255);
const BLACK: Color32 = Color32::from_rgb(0, 0, 0);

/// Coupe proprement une chaîne (nombre de caractères) avec ellipse.
/// Le texte intégral reste disponible via infobulle côté appelant.
pub fn ellipsize(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_owned()
    } else {
        let cut: String = s.chars().take(max).collect();
        format!("{cut}…")
    }
}

/// Ombre douce réutilisable (cartes, boutons actifs).
fn soft_shadow(alpha: u8) -> egui::Shadow {
    egui::Shadow {
        offset: [0, 1],
        blur: 4,
        spread: 0,
        color: Color32::from_black_alpha(alpha),
    }
}

// ─── Boutons (esprit uikit : états survol/appui, ombre discrète) ─────────────

/// Variantes visuelles de bouton.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BtnKind {
    /// Action principale (accent plein).
    Primary,
    /// Action destructive (danger plein).
    Danger,
    /// Action secondaire (contour discret).
    Ghost,
    /// Action tertiaire (fond doux, sans contour).
    Soft,
}

/// Bouton dessiné sur mesure : états survol/appui, ombre portée discrète,
/// rayon homogène. Sobre : aucune couleur hors sémantique.
pub fn styled_button(ui: &mut Ui, p: &Palette, label: &str, kind: BtnKind) -> egui::Response {
    let enabled = ui.is_enabled();
    let text_color = match kind {
        BtnKind::Primary | BtnKind::Danger => WHITE,
        _ => p.text,
    };
    let font = FontId::new(14.0, egui::FontFamily::Name("Medium".into()));
    let galley = ui.painter().layout_no_wrap(
        label.to_owned(),
        font,
        if enabled { text_color } else { p.text_secondary },
    );
    let pad = egui::vec2(16.0, 9.0);
    let size = galley.size() + 2.0 * pad;
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(size, sense);
    resp.widget_info(|| {
        egui::WidgetInfo::labeled(egui::Role::Button, enabled, label)
    });

    let hovered = enabled && resp.hovered();
    let pressed = enabled && resp.is_pointer_button_down_on();
    let base_fill = match kind {
        BtnKind::Primary => p.accent,
        BtnKind::Danger => p.danger,
        BtnKind::Ghost => Color32::TRANSPARENT,
        BtnKind::Soft => p.card_hover,
    };
    let (fill, stroke) = match kind {
        BtnKind::Ghost => (
            if pressed {
                blend(p.card_hover, p.border, 0.35)
            } else if hovered {
                p.card_hover
            } else {
                Color32::TRANSPARENT
            },
            Stroke::new(1.0, if hovered { p.accent } else { p.border }),
        ),
        _ => {
            let fill = if pressed {
                blend(base_fill, BLACK, if p.dark { 0.20 } else { 0.14 })
            } else if hovered {
                blend(base_fill, WHITE, if p.dark { 0.12 } else { 0.07 })
            } else {
                base_fill
            };
            (fill, Stroke::NONE)
        }
    };
    let radius = CornerRadius::same(10);

    // Ombre portée douce sous les boutons pleins au survol.
    if hovered && !matches!(kind, BtnKind::Ghost) {
        let alpha = if p.dark { 70 } else { 45 };
        let below = rect.translate(egui::vec2(0.0, 2.0));
        ui.painter().add(
            egui::epaint::RectShape::filled(below, radius, Color32::from_black_alpha(alpha))
                .with_blur_width(6.0),
        );
    }
    ui.painter().rect(rect, radius, fill, stroke, StrokeKind::Inside);
    // Reflet très discret sur la moitié supérieure des boutons pleins.
    if !matches!(kind, BtnKind::Ghost | BtnKind::Soft) {
        let sheen = egui::Rect::from_min_size(
            rect.left_top(),
            egui::vec2(rect.width(), (rect.height() * 0.5).max(1.0)),
        );
        let top = egui::CornerRadius { nw: 10, ne: 10, sw: 0, se: 0 };
        ui.painter().rect(
            sheen,
            top,
            Color32::from_white_alpha(if p.dark { 12 } else { 16 }),
            Stroke::NONE,
            StrokeKind::Inside,
        );
    }
    let tp = rect.center() - galley.size() * 0.5;
    ui.painter().galley(tp, galley, text_color);
    resp
}

/// Carte avec en-tête (titre majuscules espacées + actions à droite).
pub fn card<R>(
    ui: &mut Ui,
    p: &Palette,
    title: &str,
    actions: impl FnOnce(&mut Ui),
    body: impl FnOnce(&mut Ui) -> R,
) -> R {
    card_frame(ui, p, |ui| {
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
}

/// Cadre de carte : fond, bordure, rayon et ombre discrète.
fn card_frame<R>(ui: &mut Ui, p: &Palette, body: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::NONE
        .fill(p.card)
        .stroke(egui::Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(Margin::same(16))
        .shadow(soft_shadow(if p.dark { 55 } else { 20 }))
        .show(ui, body)
        .inner
}

/// Carte sans en-tête, occupant la largeur disponible.
pub fn card_body<R>(ui: &mut Ui, p: &Palette, body: impl FnOnce(&mut Ui) -> R) -> R {
    card_frame(ui, p, |ui| {
        ui.set_min_width(ui.available_width() - 2.0);
        body(ui)
    })
}

/// Interrupteur à bascule (52×28 px, animé, anneau au survol).
pub fn switch(ui: &mut Ui, p: &Palette, on: bool, accent: Option<Color32>) -> bool {
    let id = ui.scope_id().with("switch");
    let animated = ui.ctx().animate_bool(id, on);
    let size = egui::vec2(52.0, 28.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    resp.widget_info(|| {
        egui::WidgetInfo::selected(egui::Role::CheckBox, ui.is_enabled(), on, "")
    });
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
    let knob = egui::Rect::from_min_size(
        egui::pos2(x, rect.center().y - 11.0),
        egui::vec2(22.0, 22.0),
    );
    // Anneau de survol + ombre du curseur.
    if resp.hovered() {
        ui.painter().circle_filled(
            knob.center(),
            13.0,
            Color32::from_black_alpha(if p.dark { 60 } else { 30 }),
        );
    }
    ui.painter().add(
        egui::epaint::RectShape::filled(knob.translate(egui::vec2(0.0, 1.0)), CornerRadius::same(11), Color32::from_black_alpha(if p.dark { 80 } else { 45 }))
            .with_blur_width(4.0),
    );
    ui.painter().circle_filled(knob.center(), 11.0, Color32::WHITE);
    if resp.clicked() {
        !on
    } else {
        on
    }
}

/// Sélecteur segmenté en pastille (segment actif : accent + ombre douce).
/// Retourne vrai si la valeur a changé.
pub fn segmented<T: PartialEq + Copy>(
    ui: &mut Ui,
    p: &Palette,
    variants: &[(T, &str)],
    value: &mut T,
) -> bool {
    let mut changed = false;
    Frame::NONE
        .fill(p.row_alt)
        .corner_radius(CornerRadius::same(11))
        .inner_margin(Margin::same(3))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(2.0, 0.0);
            for (v, label) in variants {
                let selected = *value == *v;
                let text_col = if selected { WHITE } else { p.text_secondary };
                let font = FontId::new(12.5, egui::FontFamily::Name("Medium".into()));
                let galley = ui.painter().layout_no_wrap(label.to_string(), font, text_col);
                let w = galley.size().x + 22.0;
                let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, 26.0), Sense::click());
                resp.widget_info(|| {
                    egui::WidgetInfo::selected(egui::Role::Button, true, selected, label)
                });
                if selected {
                    let below = rect.translate(egui::vec2(0.0, 1.0));
                    ui.painter().add(
                        egui::epaint::RectShape::filled(
                            below,
                            CornerRadius::same(9),
                            Color32::from_black_alpha(if p.dark { 55 } else { 28 }),
                        )
                        .with_blur_width(4.0),
                    );
                }
                let fill = if selected {
                    p.accent
                } else if resp.hovered() {
                    p.card_hover
                } else {
                    Color32::TRANSPARENT
                };
                ui.painter().rect(rect, CornerRadius::same(9), fill, Stroke::NONE, StrokeKind::Inside);
                ui.painter().galley(rect.center() - galley.size() * 0.5, galley, text_col);
                if resp.clicked() && !selected {
                    *value = *v;
                    changed = true;
                }
            }
        })
        .inner;
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
    styled_button(ui, p, label, BtnKind::Primary)
}

/// Bouton stylé avec état activé/désactivé explicite (grisé si désactivé).
pub fn styled_button_en(
    ui: &mut Ui,
    p: &Palette,
    label: &str,
    kind: BtnKind,
    enabled: bool,
) -> egui::Response {
    if enabled {
        styled_button(ui, p, label, kind)
    } else {
        let mut child = ui.new_child(egui::UiBuilder::new().disabled());
        styled_button(&mut child, p, label, kind)
    }
}

/// Petit bouton principal avec icône (« + Ajouter »).
pub fn primary_button_small(ui: &mut Ui, p: &Palette, icon: &str, label: &str) -> egui::Response {
    styled_button(ui, p, &format!("{}  {}", icon, label), BtnKind::Primary)
}

/// Bouton destructif (danger plein).
pub fn danger_button(ui: &mut Ui, p: &Palette, label: &str) -> egui::Response {
    styled_button(ui, p, label, BtnKind::Danger)
}

/// Bouton secondaire (contour discret).
pub fn ghost_button(ui: &mut Ui, p: &Palette, label: &str) -> egui::Response {
    styled_button(ui, p, label, BtnKind::Ghost)
}

/// Bouton icône discret, libellé en infobulle.
pub fn icon_button(ui: &mut Ui, p: &Palette, icon: &str, tooltip: &str) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), Sense::click());
    let hovered = resp.hovered();
    let pressed = resp.is_pointer_button_down_on();
    let fill = if pressed {
        blend(p.card_hover, p.border, 0.35)
    } else if hovered {
        p.card_hover
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect(rect, CornerRadius::same(8), fill, Stroke::NONE, StrokeKind::Inside);
    let col = if hovered { p.text } else { p.text_secondary };
    let font = FontId::new(17.0, egui::FontFamily::Name("Icon".into()));
    let galley = ui.painter().layout_no_wrap(icon.to_string(), font, col);
    ui.painter().galley(rect.center() - galley.size() * 0.5, galley, col);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tooltip));
    resp.on_hover_text(tooltip)
}

/// Texte riche combinant icône et libellé (pour boutons).
/// L'icône est résolue par repli de police (Material Icons en fin de liste) ;
/// le libellé utilise la police texte.
pub fn btn_icon_text(icon: &str, label: &str) -> RichText {
    RichText::new(format!("{}  {}", icon, label))
        .family(egui::FontFamily::Name("Medium".into()))
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

/// Affiche l'ancienneté des données (traduit).
pub fn since_label(ui: &mut Ui, p: &Palette, t: &crate::i18n::Tr, at: std::time::Instant) {
    let txt = t.since(at.elapsed().as_secs());
    ui.label(RichText::new(txt).size(11.5).color(p.text_secondary));
}

/// Fond de modale (frame utilisée par crate::modal).
pub fn modal_frame(p: &Palette) -> egui::Frame {
    Frame::NONE
        .fill(p.card)
        .stroke(egui::Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(14))
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

