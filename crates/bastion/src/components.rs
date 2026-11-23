//! Composants d'interface : cartes, boutons, interrupteurs, sélecteurs,
//! badges, bandeaux, tuiles de chiffres. Tout ce qui donne à l'application
//! son langage visuel sobre et cohérent (voir docs/ERGONOMIE.md).
//!
//! Règles du langage :
//!  - un bouton a toujours la même hauteur (34 px) et le même rayon ;
//!  - un seul bouton plein par zone (l'action principale) ;
//!  - les couleurs sémantiques ne colorent que les états, jamais la décoration.

use crate::icons;
use crate::theme::{shape, space, text, Palette};
use egui::{
    Align, Color32, CornerRadius, FontId, Frame, Layout, Margin, Response, RichText, Sense,
    Stroke, StrokeKind, Ui, Vec2,
};

/// Mélange une couleur vers un fond (k = part de la couleur).
pub fn blend(c: Color32, bg: Color32, k: f32) -> Color32 {
    let mix = |a: u8, b: u8| -> u8 { ((a as f32) * (1.0 - k) + (b as f32) * k) as u8 };
    Color32::from_rgb(mix(bg.r(), c.r()), mix(bg.g(), c.g()), mix(bg.b(), c.b()))
}

const WHITE: Color32 = Color32::from_rgb(255, 255, 255);
const BLACK: Color32 = Color32::from_rgb(0, 0, 0);

/// Couleur de texte lisible sur un aplat (blanc ou encre), au choix le plus
/// contrasté — indispensable pour les boutons pleins en thème sombre.
fn readable_on(bg: Color32, p: &Palette) -> Color32 {
    let lum = |c: Color32| {
        let f = |v: u8| {
            let v = v as f32 / 255.0;
            if v <= 0.03928 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * f(c.r()) + 0.7152 * f(c.g()) + 0.0722 * f(c.b())
    };
    let white = lum(WHITE);
    let ink = lum(p.on_accent);
    let bg_l = lum(bg);
    let c_white = (white + 0.05) / (bg_l + 0.05);
    let c_ink = (bg_l + 0.05) / (ink + 0.05);
    if c_white >= c_ink {
        WHITE
    } else {
        p.on_accent
    }
}

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

// ─── Boutons ─────────────────────────────────────────────────────────────────

/// Variantes visuelles de bouton.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BtnKind {
    /// Action principale (aplat d'accent).
    Primary,
    /// Action destructive (aplat danger).
    Danger,
    /// Action secondaire (contour discret).
    Ghost,
    /// Action tertiaire (fond doux, sans contour).
    Soft,
    /// Action discrète (texte seul, sans fond).
    Quiet,
}

/// Bouton dessiné sur mesure : états survol/appui, anneau de focus clavier,
/// rayon et hauteur homogènes.
pub fn styled_button(ui: &mut Ui, p: &Palette, label: &str, kind: BtnKind) -> Response {
    styled_button_opts(ui, p, label, kind, ui.is_enabled(), 0.0)
}

/// Bouton stylé avec état activé/désactivé et largeur minimale explicites.
pub fn styled_button_minw(
    ui: &mut Ui,
    p: &Palette,
    label: &str,
    kind: BtnKind,
    enabled: bool,
    min_width: f32,
) -> Response {
    styled_button_opts(ui, p, label, kind, enabled, min_width)
}

/// Bouton stylé avec état activé/désactivé explicite (grisé si désactivé).
pub fn styled_button_en(
    ui: &mut Ui,
    p: &Palette,
    label: &str,
    kind: BtnKind,
    enabled: bool,
) -> Response {
    styled_button_opts(ui, p, label, kind, enabled, 0.0)
}

/// Bouton pleine largeur (formulaires, bandeaux).
pub fn styled_button_fill(
    ui: &mut Ui,
    p: &Palette,
    label: &str,
    kind: BtnKind,
    enabled: bool,
) -> Response {
    let w = ui.available_width();
    styled_button_opts(ui, p, label, kind, enabled, w)
}

fn styled_button_opts(
    ui: &mut Ui,
    p: &Palette,
    label: &str,
    kind: BtnKind,
    enabled: bool,
    min_width: f32,
) -> Response {
    let fg = if enabled {
        match kind {
            BtnKind::Primary => readable_on(p.accent, p),
            BtnKind::Danger => readable_on(p.danger, p),
            BtnKind::Ghost | BtnKind::Soft => p.text,
            BtnKind::Quiet => p.text_secondary,
        }
    } else {
        blend(p.text_secondary, p.card, 0.35)
    };
    let font = FontId::new(text::LABEL, egui::FontFamily::Name("Medium".into()));
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font, fg);
    let pad_x = 14.0;
    let mut size = Vec2::new(galley.size().x + 2.0 * pad_x, shape::BTN_H);
    size.x = size.x.max(min_width);
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(size, sense);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, label));

    let hovered = enabled && resp.hovered();
    let pressed = enabled && resp.is_pointer_button_down_on();
    let (base, hover, press) = match kind {
        BtnKind::Primary => (p.accent, p.accent_hover, blend(p.accent_hover, BLACK, 0.12)),
        BtnKind::Danger => (p.danger, blend(p.danger, BLACK, 0.12), blend(p.danger, BLACK, 0.22)),
        BtnKind::Ghost => (Color32::TRANSPARENT, p.card_hover, blend(p.card_hover, BLACK, 0.08)),
        BtnKind::Soft => (p.card_hover, blend(p.card_hover, p.border_strong, 0.45), blend(p.card_hover, p.border_strong, 0.7)),
        BtnKind::Quiet => (Color32::TRANSPARENT, p.card_hover, blend(p.card_hover, BLACK, 0.08)),
    };
    let fill = if !enabled {
        // Un bouton principal inactif ne doit pas ressembler à une barre de
        // progression : fond neutre, contour discret, texte atténué.
        match kind {
            BtnKind::Primary | BtnKind::Danger => blend(p.card_hover, p.card, 0.35),
            _ => Color32::TRANSPARENT,
        }
    } else if pressed {
        press
    } else if hovered {
        hover
    } else {
        base
    };
    let stroke = match kind {
        BtnKind::Ghost => Stroke::new(1.0, if hovered { p.border_strong } else { p.border }),
        BtnKind::Primary | BtnKind::Danger if !enabled => Stroke::new(1.0, p.border),
        BtnKind::Primary | BtnKind::Danger => Stroke::NONE,
        _ => Stroke::NONE,
    };
    let radius = CornerRadius::same(shape::R_BUTTON);
    ui.painter().rect(rect, radius, fill, stroke, StrokeKind::Inside);
    // Anneau de focus clavier (l'activation Entrée/Espace est gérée par egui
    // pour tout widget cliquable).
    if resp.has_focus() {
        ui.painter().rect_stroke(
            rect.expand(2.0),
            CornerRadius::same(shape::R_BUTTON + 2),
            Stroke::new(2.0, p.accent),
            StrokeKind::Outside,
        );
    }
    let tp = rect.center() - galley.size() * 0.5;
    ui.painter().galley(tp, galley, fg);
    resp
}

/// Bouton principal (accent plein).
pub fn primary_button(ui: &mut Ui, p: &Palette, label: &str) -> Response {
    styled_button(ui, p, label, BtnKind::Primary)
}

/// Bouton destructif (danger plein).
pub fn danger_button(ui: &mut Ui, p: &Palette, label: &str) -> Response {
    styled_button(ui, p, label, BtnKind::Danger)
}

/// Bouton secondaire (contour discret).
pub fn ghost_button(ui: &mut Ui, p: &Palette, label: &str) -> Response {
    styled_button(ui, p, label, BtnKind::Ghost)
}

/// Bouton tertiaire (fond doux).
pub fn soft_button(ui: &mut Ui, p: &Palette, label: &str) -> Response {
    styled_button(ui, p, label, BtnKind::Soft)
}

/// Bouton icône discret, libellé en infobulle.
pub fn icon_button(ui: &mut Ui, p: &Palette, icon: &str, tooltip: &str) -> Response {
    icon_button_sz(ui, p, icon, tooltip, 30.0, 16.0)
}

/// Bouton icône de taille choisie (les lignes de tableau utilisent 26 px).
pub fn icon_button_sz(
    ui: &mut Ui,
    p: &Palette,
    icon: &str,
    tooltip: &str,
    side: f32,
    glyph: f32,
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(side), Sense::click());
    let hovered = resp.hovered();
    let pressed = resp.is_pointer_button_down_on();
    let fill = if pressed {
        blend(p.card_hover, p.border, 0.4)
    } else if hovered {
        p.card_hover
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect(
        rect,
        CornerRadius::same(shape::R_BUTTON),
        fill,
        Stroke::NONE,
        StrokeKind::Inside,
    );
    let col = if hovered { p.text } else { p.text_secondary };
    let font = FontId::new(glyph, egui::FontFamily::Name("Icon".into()));
    let galley = ui.painter().layout_no_wrap(icon.to_string(), font, col);
    ui.painter().galley(rect.center() - galley.size() * 0.5, galley, col);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tooltip));
    resp.on_hover_text(tooltip)
}

/// Petit bouton icône destructif (suppression dans une ligne).
pub fn icon_danger_button(ui: &mut Ui, p: &Palette, icon: &str, tooltip: &str) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(26.0), Sense::click());
    let hovered = resp.hovered();
    if hovered {
        ui.painter().rect(
            rect,
            CornerRadius::same(shape::R_BUTTON),
            p.soft(p.danger),
            Stroke::NONE,
            StrokeKind::Inside,
        );
    }
    let col = if hovered { p.danger } else { p.text_secondary };
    let font = FontId::new(15.0, egui::FontFamily::Name("Icon".into()));
    let galley = ui.painter().layout_no_wrap(icon.to_string(), font, col);
    ui.painter().galley(rect.center() - galley.size() * 0.5, galley, col);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tooltip));
    resp.on_hover_text(tooltip)
}

// ─── Cartes ──────────────────────────────────────────────────────────────────

/// Carte avec en-tête (titre + actions à droite).
pub fn card<R>(
    ui: &mut Ui,
    p: &Palette,
    title: &str,
    actions: impl FnOnce(&mut Ui),
    body: impl FnOnce(&mut Ui) -> R,
) -> R {
    card_frame(ui, p, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(title)
                    .size(text::TITLE)
                    .color(p.text)
                    .family(egui::FontFamily::Name("SemiBold".into())),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.style_mut().spacing.item_spacing = Vec2::new(6.0, 6.0);
                actions(ui);
            });
        });
        ui.add_space(space::M);
        body(ui)
    })
}

/// Cadre de carte : fond, bordure, rayon. Plat, sans ombre superflue.
fn card_frame<R>(ui: &mut Ui, p: &Palette, body: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::NONE
        .fill(p.card)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(shape::R_CARD))
        .inner_margin(Margin::same(16))
        .show(ui, body)
        .inner
}

/// Carte sans en-tête, occupant la largeur disponible.
pub fn card_body<R>(ui: &mut Ui, p: &Palette, body: impl FnOnce(&mut Ui) -> R) -> R {
    card_frame(ui, p, |ui| {
        ui.set_width(ui.available_width());
        body(ui)
    })
}

/// Sous-titre de section à l'intérieur d'une carte.
pub fn section(ui: &mut Ui, p: &Palette, label: &str) {
    ui.label(
        RichText::new(label.to_uppercase())
            .size(text::MICRO)
            .color(p.text_secondary)
            .family(egui::FontFamily::Name("Medium".into())),
    );
}

/// Filet horizontal discret.
pub fn hairline(ui: &mut Ui, p: &Palette) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::ZERO, p.border);
}

// ─── Contrôles ───────────────────────────────────────────────────────────────

/// Interrupteur avec état activé/désactivé.
pub fn switch_enabled(
    ui: &mut Ui,
    p: &Palette,
    on: bool,
    accent: Option<Color32>,
    enabled: bool,
) -> bool {
    let size = Vec2::new(46.0, 26.0);
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(size, sense);
    // L'identifiant du widget (unique, stable d'une image à l'autre) porte
    // l'animation : `scope_id` est partagé par tous les sous-Ui voisins.
    let animated = ui.ctx().animate_bool(resp.id, on);
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::CheckBox, enabled, on, ""));
    let track = CornerRadius::same(13);
    let base = accent.unwrap_or(p.success);
    let bg = if !enabled {
        blend(p.card_hover, p.card, 0.4)
    } else if on {
        base
    } else {
        blend(p.card_hover, p.border_strong, 0.35)
    };
    ui.painter().rect_filled(rect, track, bg);
    let travel = rect.width() - 26.0;
    let x = rect.left() + 2.0 + animated * travel;
    let knob = egui::Rect::from_min_size(
        egui::pos2(x, rect.center().y - 11.0),
        Vec2::splat(22.0),
    );
    if resp.hovered() && enabled {
        ui.painter().circle_filled(
            knob.center(),
            13.0,
            Color32::from_black_alpha(if p.dark { 70 } else { 26 }),
        );
    }
    ui.painter().circle_filled(knob.center(), 11.0, Color32::WHITE);
    if resp.has_focus() {
        ui.painter().rect_stroke(
            rect.expand(2.0),
            CornerRadius::same(15),
            Stroke::new(2.0, p.accent),
            StrokeKind::Outside,
        );
    }
    if resp.clicked() && enabled {
        !on
    } else {
        on
    }
}

/// Ligne « interrupteur + libellé + description ».
pub fn switch_row(
    ui: &mut Ui,
    p: &Palette,
    on: bool,
    enabled: bool,
    label: &str,
    desc: &str,
) -> bool {
    let mut value = on;
    ui.horizontal(|ui| {
        let w = ui.available_width();
        let switch_w = 54.0;
        ui.allocate_ui_with_layout(
            Vec2::new((w - switch_w).max(120.0), 0.0),
            Layout::top_down(Align::Min),
            |ui| {
                ui.label(
                    RichText::new(label)
                        .size(text::LABEL)
                        .color(p.text)
                        .family(egui::FontFamily::Name("Medium".into())),
                );
                if !desc.is_empty() {
                    ui.label(RichText::new(desc).size(text::SMALL).color(p.text_secondary));
                }
            },
        );
        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
            value = switch_enabled(ui, p, on, None, enabled);
        });
    });
    value
}

/// Sélecteur segmenté en pastille. Retourne vrai si la valeur a changé.
pub fn segmented<T: PartialEq + Copy>(
    ui: &mut Ui,
    p: &Palette,
    variants: &[(T, &str)],
    value: &mut T,
) -> bool {
    segmented_opts(ui, p, variants, value, false)
}

/// Sélecteur segmenté réparti sur toute la largeur (formulaires).
pub fn segmented_fill<T: PartialEq + Copy>(
    ui: &mut Ui,
    p: &Palette,
    variants: &[(T, &str)],
    value: &mut T,
) -> bool {
    segmented_opts(ui, p, variants, value, true)
}

fn segmented_opts<T: PartialEq + Copy>(
    ui: &mut Ui,
    p: &Palette,
    variants: &[(T, &str)],
    value: &mut T,
    expand: bool,
) -> bool {
    let mut changed = false;
    let n = variants.len().max(1) as f32;
    let inner_w = (ui.available_width() - 6.0).max(40.0);
    let seg_w = ((inner_w - 3.0 * (n - 1.0)) / n).max(28.0);
    Frame::NONE
        .fill(p.field)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(3))
        .show(ui, |ui| {
            // Toujours en ligne : l'appelant peut être dans une disposition
            // verticale (formulaires) sans empiler les segments.
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);
                for (v, label) in variants {
                    let selected = *value == *v;
                    let fg = if selected {
                        readable_on(p.accent, p)
                    } else {
                        p.text_secondary
                    };
                    let font = FontId::new(text::LABEL, egui::FontFamily::Name("Medium".into()));
                    let galley = ui.painter().layout_no_wrap(label.to_string(), font, fg);
                    let w = if expand {
                        seg_w
                    } else {
                        (galley.size().x + 22.0).max(44.0)
                    };
                    let (rect, resp) =
                        ui.allocate_exact_size(Vec2::new(w, 26.0), Sense::click());
                    resp.widget_info(|| {
                        egui::WidgetInfo::selected(egui::Role::Button, true, selected, label)
                    });
                    if selected {
                        ui.painter()
                            .rect_filled(rect, CornerRadius::same(8), p.accent);
                    } else if resp.hovered() {
                        ui.painter()
                            .rect_filled(rect, CornerRadius::same(8), p.card_hover);
                    }
                    if resp.has_focus() {
                        ui.painter().rect_stroke(
                            rect,
                            CornerRadius::same(8),
                            Stroke::new(2.0, p.accent),
                            StrokeKind::Inside,
                        );
                    }
                    ui.painter()
                        .galley(rect.center() - galley.size() * 0.5, galley, fg);
                    if resp.clicked() && !selected {
                        *value = *v;
                        changed = true;
                    }
                }
            });
        });
    changed
}

// ─── Badges, puces, indicateurs ──────────────────────────────────────────────

/// Badge coloré (état, action…).
pub fn badge(ui: &mut Ui, _p: &Palette, text: &str, fg: Color32, bg: Color32) {
    Frame::NONE
        .fill(bg)
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(
                RichText::new(text)
                    .size(text::MICRO)
                    .color(fg)
                    .family(egui::FontFamily::Name("Medium".into())),
            );
        });
}

/// Badge d'état : fond doux dérivé de la teinte, texte contrasté.
pub fn tag(ui: &mut Ui, p: &Palette, text: &str, tone: Color32) {
    badge(ui, p, text, tone, p.soft(tone));
}

/// Badge d'action sémantique (Autoriser / Bloquer / Rejeter / Limiter).
pub fn action_badge(ui: &mut Ui, p: &Palette, text: &str, color: Color32) {
    badge(ui, p, text, color, p.soft(color));
}

/// Point d'état coloré.
pub fn status_dot(ui: &mut Ui, color: Color32, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size + 6.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), size * 0.5, color);
}

/// Point d'état + libellé (+ précision secondaire).
pub fn status_line(ui: &mut Ui, p: &Palette, color: Color32, label: &str, sub: Option<&str>) {
    ui.horizontal(|ui| {
        status_dot(ui, color, 9.0);
        ui.label(
            RichText::new(label)
                .size(text::BODY + 2.0)
                .color(p.text)
                .family(egui::FontFamily::Name("SemiBold".into())),
        );
        if let Some(s) = sub {
            ui.label(RichText::new(s).size(text::SMALL).color(p.text_secondary));
        }
    });
}

/// Puces de filtre / de sélection rapide.
pub fn chip(ui: &mut Ui, p: &Palette, label: &str, selected: bool) -> Response {
    let fg = if selected {
        readable_on(p.accent, p)
    } else {
        p.text_secondary
    };
    let font = FontId::new(text::LABEL, egui::FontFamily::Name("Medium".into()));
    let galley = ui.painter().layout_no_wrap(label.to_string(), font, fg);
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(galley.size().x + 24.0, 28.0), Sense::click());
    resp.widget_info(|| {
        egui::WidgetInfo::selected(egui::Role::Button, true, selected, label)
    });
    let fill = if selected {
        p.accent
    } else if resp.hovered() {
        p.card_hover
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect(
        rect,
        CornerRadius::same(14),
        fill,
        Stroke::new(1.0, if selected { p.accent } else { p.border }),
        StrokeKind::Inside,
    );
    ui.painter()
        .galley(rect.center() - galley.size() * 0.5, galley, fg);
    resp
}

/// Tuile de chiffre clé : icône teintée, valeur, libellé.
pub fn stat_tile(
    ui: &mut Ui,
    p: &Palette,
    icon: &str,
    tone: Color32,
    value: &str,
    label: &str,
) {
    Frame::NONE
        .fill(p.card)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(shape::R_CARD))
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(36.0), Sense::hover());
                ui.painter().rect_filled(rect, CornerRadius::same(10), p.soft(tone));
                let galley = ui.painter().layout_no_wrap(
                    icon.to_string(),
                    FontId::new(19.0, egui::FontFamily::Name("Icon".into())),
                    tone,
                );
                ui.painter()
                    .galley(rect.center() - galley.size() * 0.5, galley, tone);
                ui.add_space(2.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(value)
                            .size(22.0)
                            .color(p.text)
                            .family(egui::FontFamily::Name("SemiBold".into())),
                    );
                    ui.label(RichText::new(label).size(text::MICRO).color(p.text_secondary));
                });
            });
        });
}

/// Bandeau d'alerte / d'information avec action optionnelle.
pub fn banner<R>(
    ui: &mut Ui,
    p: &Palette,
    tone: Color32,
    icon: &str,
    title: &str,
    body: &str,
    action: impl FnOnce(&mut Ui) -> R,
) {
    Frame::NONE
        .fill(p.soft(tone))
        .stroke(Stroke::new(1.0, blend(tone, p.card, 0.45)))
        .corner_radius(CornerRadius::same(11))
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(icons::icon_sz(icon, 18.0, tone));
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(title)
                            .size(text::LABEL)
                            .color(p.text)
                            .family(egui::FontFamily::Name("SemiBold".into())),
                    );
                    if !body.is_empty() {
                        ui.label(RichText::new(body).size(text::SMALL).color(p.text_secondary));
                    }
                });
            });
            action(ui);
        });
}

// ─── Formulaires ─────────────────────────────────────────────────────────────

/// Libellé de champ.
fn field_label(ui: &mut Ui, p: &Palette, label: &str) {
    ui.label(
        RichText::new(label)
            .size(text::SMALL)
            .color(p.text_secondary)
            .family(egui::FontFamily::Name("Medium".into())),
    );
}

/// Libellé de champ + saisie monoligne.
pub fn field(ui: &mut Ui, p: &Palette, label: &str, value: &mut String, hint: &str) -> Response {
    let w = ui.available_width();
    field_sized(ui, p, label, value, hint, w)
}

/// Champ de largeur imposée (lignes à plusieurs champs).
pub fn field_sized(
    ui: &mut Ui,
    p: &Palette,
    label: &str,
    value: &mut String,
    hint: &str,
    width: f32,
) -> Response {
    ui.vertical(|ui| {
        field_label(ui, p, label);
        let edit = egui::TextEdit::singleline(value)
            .hint_text(hint)
            .desired_width(width)
            .margin(Margin::symmetric(10, 8))
            .align(egui::Align2::LEFT_CENTER);
        let resp = ui.add(edit);
        if resp.has_focus() {
            ui.painter().rect_stroke(
                resp.rect.expand(1.0),
                CornerRadius::same(shape::R_INPUT),
                Stroke::new(2.0, p.accent),
                StrokeKind::Outside,
            );
        }
        resp
    })
    .inner
}

/// Champ mot de passe avec révélation temporaire (œil).
pub fn password_field(ui: &mut Ui, p: &Palette, label: &str, value: &mut String) -> Response {
    let id = ui.make_persistent_id(("pw-reveal", label));
    let mut revealed = ui.data(|d| d.get_temp::<bool>(id).unwrap_or(false));
    let mut edit_resp = None;
    ui.vertical(|ui| {
        field_label(ui, p, label);
        ui.horizontal(|ui| {
            let eye_w = 30.0;
            let edit = egui::TextEdit::singleline(value)
                .password(!revealed)
                .desired_width((ui.available_width() - eye_w - 6.0).max(80.0))
                .margin(Margin::symmetric(10, 8))
                .align(egui::Align2::LEFT_CENTER);
            edit_resp = Some(ui.add(edit));
            let icon = if revealed { icons::VISIBILITY_OFF } else { icons::VISIBILITY };
            if icon_button_sz(ui, p, icon, label, 28.0, 15.0).clicked() {
                revealed = !revealed;
                ui.data_mut(|d| d.insert_temp(id, revealed));
            }
        });
    });
    edit_resp.expect("champ de mot de passe")
}

/// Champ de recherche avec icône et bouton d'effacement.
pub fn search_field(ui: &mut Ui, p: &Palette, value: &mut String, hint: &str) -> Response {
    let mut resp = None;
    Frame::NONE
        .fill(p.field)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(shape::R_INPUT))
        .inner_margin(Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(icons::icon_sz(icons::SEARCH, 15.0, p.text_secondary));
                let clear_w = if value.is_empty() { 0.0 } else { 26.0 };
                let edit = egui::TextEdit::singleline(value)
                    .hint_text(hint)
                    .desired_width((ui.available_width() - clear_w).max(60.0))
                    .frame(egui::Frame::NONE)
                    .margin(Margin::symmetric(2, 4))
                    .align(egui::Align2::LEFT_CENTER);
                resp = Some(ui.add(edit));
                if !value.is_empty()
                    && icon_button_sz(ui, p, icons::CLOSE, "", 24.0, 13.0).clicked()
                {
                    value.clear();
                }
            });
        });
    resp.expect("champ de recherche")
}

/// Aide contextuelle (texte secondaire).
pub fn hint(ui: &mut Ui, p: &Palette, text: &str) {
    ui.label(RichText::new(text).size(text::SMALL).color(p.text_secondary));
}

/// Aide posée sur le fond de travail.
pub fn hint_on_bg(ui: &mut Ui, p: &Palette, text: &str) {
    ui.label(
        RichText::new(text)
            .size(text::SMALL)
            .color(p.on_bg_secondary),
    );
}

/// Lien d'action discret (accent, sans fond).
pub fn link_button(ui: &mut Ui, p: &Palette, label: &str) -> Response {
    let font = FontId::new(text::SMALL, egui::FontFamily::Name("Medium".into()));
    let galley = ui.painter().layout_no_wrap(label.to_string(), font.clone(), p.accent);
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(galley.size().x + 4.0, 20.0), Sense::click());
    let col = if resp.hovered() {
        p.accent_hover
    } else {
        p.accent
    };
    let galley = ui.painter().layout_no_wrap(label.to_string(), font, col);
    ui.painter()
        .galley(rect.center() - galley.size() * 0.5, galley, col);
    resp
}

// ─── États ───────────────────────────────────────────────────────────────────

/// État vide (aucune donnée) avec message et suggestion.
pub fn empty_state(ui: &mut Ui, p: &Palette, message: &str, sub: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(18.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(44.0), Sense::hover());
        ui.painter()
            .circle_filled(rect.center(), 22.0, p.soft(p.text_secondary));
        let galley = ui.painter().layout_no_wrap(
            icons::INBOX.to_string(),
            FontId::new(21.0, egui::FontFamily::Name("Icon".into())),
            p.text_secondary,
        );
        ui.painter()
            .galley(rect.center() - galley.size() * 0.5, galley, p.text_secondary);
        ui.add_space(8.0);
        ui.label(
            RichText::new(message)
                .size(text::BODY)
                .color(p.text)
                .family(egui::FontFamily::Name("Medium".into())),
        );
        ui.label(RichText::new(sub).size(text::SMALL).color(p.text_secondary));
        ui.add_space(10.0);
    });
}

/// État vide avec une action (bouton centré).
pub fn empty_state_action(
    ui: &mut Ui,
    p: &Palette,
    message: &str,
    sub: &str,
    action: &str,
) -> Response {
    empty_state(ui, p, message, sub);
    let resp = ui
        .horizontal(|ui| {
            let w = ui.available_width();
            ui.add_space((w - 180.0).max(0.0) * 0.5);
            primary_button(ui, p, action)
        })
        .inner;
    ui.add_space(8.0);
    resp
}

/// Écran vide « hors connexion » avec un bouton vers l'écran Connexion.
pub fn need_connection(ui: &mut Ui, p: &Palette, t: &crate::i18n::Tr) -> Response {
    empty_state_action(ui, p, t.status_disconnected, t.need_connection_hint, t.go_connect)
}

/// Spinner de chargement discret + libellé.
pub fn loading(ui: &mut Ui, p: &Palette, label: &str) {
    ui.horizontal(|ui| {
        ui.add(egui::Spinner::new().size(14.0));
        ui.label(RichText::new(label).size(text::SMALL).color(p.text_secondary));
    });
}

/// Affiche l'ancienneté des données (traduit).
pub fn since_label(ui: &mut Ui, p: &Palette, t: &crate::i18n::Tr, at: std::time::Instant) {
    let txt = t.since(at.elapsed().as_secs());
    ui.label(RichText::new(txt).size(text::MICRO).color(p.text_secondary));
}

/// Fond de modale (frame utilisée par crate::modal).
pub fn modal_frame(p: &Palette) -> egui::Frame {
    Frame::NONE
        .fill(p.card)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(shape::R_MODAL))
        .inner_margin(Margin::same(20))
        .shadow(soft_shadow(if p.dark { 90 } else { 30 }))
}

/// Titre de modale (rendu dans le contenu de la modale).
pub fn modal_title(ui: &mut Ui, _p: &Palette, title: &str) {
    ui.label(
        RichText::new(title)
            .size(17.0)
            .family(egui::FontFamily::Name("SemiBold".into())),
    );
    ui.add_space(space::S);
}

// ─── Mise en page responsive (mobile-first) ──────────────────────────────────
//
// Toute l'interface est conçue d'abord pour un écran étroit (une colonne,
// navigation basse), puis s'enrichit quand la fenêtre s'élargit : les tuiles
// passent côte à côte et la navigation migre en barre latérale.

/// Largeur de fenêtre au-dessous de laquelle Bastion adopte la présentation
/// compacte (mobile) : navigation en bas d'écran, tuiles empilées.
pub const COMPACT_MAX_WIDTH: f32 = 720.0;

/// Gouttière horizontale entre colonnes de tuiles. Doit rester égale à
/// `style.spacing.item_spacing.x` (8 px) : c'est la valeur qu'egui utilise
/// entre les colonnes de `Ui::columns`.
pub const TILE_GAP_X: f32 = 8.0;

/// Espacement vertical entre deux rangées de tuiles.
pub const TILE_GAP_Y: f32 = 12.0;

/// L'interface est-elle en mode compact (fenêtre étroite / mobile) ?
pub fn is_compact(width: f32) -> bool {
    width < COMPACT_MAX_WIDTH
}

/// Nombre de colonnes de tuiles pouvant tenir dans `avail` (au moins une) :
/// chaque colonne doit mesurer au moins `min_tile_w`, gouttières comprises.
/// Fonction pure — testée unitairement.
pub fn tile_cols(avail: f32, min_tile_w: f32, max_cols: usize) -> usize {
    let mut cols = 1usize;
    for cand in 1..=max_cols {
        let per_col = (avail - TILE_GAP_X * (cand - 1) as f32) / cand as f32;
        if per_col >= min_tile_w {
            cols = cand;
        }
    }
    cols
}

/// Grille de tuiles responsive : les `tiles` tuiles se placent côte à côte
/// tant que la largeur le permet (au moins `min_tile_w` chacune), puis
/// passent à la ligne ; en fenêtre étroite tout s'empile sur une colonne.
/// `render` est appelé exactement une fois par tuile, avec l'ui de la
/// tuile et son index.
pub fn tile_grid(
    ui: &mut Ui,
    p: &Palette,
    min_tile_w: f32,
    tiles: usize,
    mut render: impl FnMut(&mut Ui, &Palette, usize),
) {
    if tiles == 0 {
        return;
    }
    let avail = ui.available_width();
    let mut cols = tile_cols(avail, min_tile_w, tiles).min(tiles);
    // Préfère un compte qui remplit des lignes entières (4 tuiles → 2×2 et non
    // 3 + 1) ; la dernière ligne garde sinon la largeur des autres tuiles.
    if cols > 2 && tiles % cols != 0 {
        if let Some(c) = (2..cols).rev().find(|c| tiles % c == 0) {
            cols = c;
        }
    }
    let mut i = 0usize;
    while i < tiles {
        let row = cols.min(tiles - i);
        ui.columns(cols, |cols_ui| {
            for (j, col) in cols_ui.iter_mut().take(row).enumerate() {
                render(col, p, i + j);
            }
        });
        i += row;
        if i < tiles {
            ui.add_space(TILE_GAP_Y);
        }
    }
}

/// Comble la dernière colonne d'un tableau jusqu'au bord droit de sa carte.
/// Une `egui::Grid` se cale sur la largeur de son contenu et laisse sinon un
/// blanc à droite ; egui expose déjà, pour la dernière colonne, comme largeur
/// disponible tout l'espace restant — il suffit de l'allouer. À appeler dans
/// la dernière cellule (au sein d'un sous-`Ui` `horizontal`/`vertical`, chaque
/// widget ajouté directement étant une cellule à part entière).
pub fn stretch_cell(ui: &mut Ui) {
    ui.allocate_exact_size(Vec2::new(ui.available_width(), 0.0), Sense::hover());
}

/// Paramètre d'interface « libellé + contrôle » : sur une même ligne tant que
/// la largeur disponible atteint `min_inline_w`, sinon empilés (libellé
/// au-dessus, contrôle pleine largeur). C'est la brique responsive de chaque
/// paramètre des écrans.
pub fn labeled_control(
    ui: &mut Ui,
    p: &Palette,
    label: &str,
    min_inline_w: f32,
    add: impl FnOnce(&mut Ui),
) {
    let label_text = RichText::new(label)
        .size(text::SMALL)
        .color(p.text_secondary)
        .family(egui::FontFamily::Name("Medium".into()));
    if ui.available_width() >= min_inline_w {
        ui.horizontal(|ui| {
            let w = (ui.available_width() * 0.42).clamp(90.0, 190.0);
            ui.allocate_ui_with_layout(Vec2::new(w, 18.0), Layout::left_to_right(Align::Center), |ui| {
                ui.label(label_text);
            });
            add(ui);
        });
    } else {
        ui.label(label_text);
        ui.add_space(2.0);
        add(ui);
    }
}

/// En-tête de colonne de tableau.
pub fn th(p: &Palette, s: &str) -> RichText {
    RichText::new(s)
        .size(text::MICRO)
        .color(p.text_secondary)
        .family(egui::FontFamily::Name("Medium".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn palette() -> Palette {
        Palette::new(crate::config::ThemeKind::Light)
    }

    /// Nombre de colonnes : respect de la largeur minimale, gouttières
    /// comprises, toujours au moins une colonne.
    #[test]
    fn tile_cols_respecte_largeur_minimale() {
        // 2 colonnes de 340 px + 8 px de gouttière → il faut 688 px.
        assert_eq!(tile_cols(300.0, 340.0, 2), 1, "300 px : une colonne");
        assert_eq!(tile_cols(600.0, 340.0, 2), 1, "600 px : une colonne");
        assert_eq!(tile_cols(700.0, 340.0, 2), 2, "700 px : deux colonnes");
        assert_eq!(tile_cols(2000.0, 340.0, 2), 2, "borné par max_cols");
        // 3 colonnes de 300 px + 2 gouttières → 916 px.
        assert_eq!(tile_cols(900.0, 300.0, 3), 2, "900 px : deux colonnes");
        assert_eq!(tile_cols(1000.0, 300.0, 3), 3, "1000 px : trois colonnes");
        // Dégradés raisonnables : jamais zéro colonne.
        assert_eq!(tile_cols(0.0, 340.0, 2), 1, "largeur nulle : une colonne");
        assert_eq!(tile_cols(500.0, 340.0, 0), 1, "max_cols nul : une colonne");
    }

    /// Mode compact : bascule exactement à COMPACT_MAX_WIDTH.
    #[test]
    fn is_compact_bascule_a_720() {
        assert!(is_compact(360.0), "360 px : mobile");
        assert!(is_compact(719.9), "juste sous le seuil : mobile");
        assert!(!is_compact(720.0), "seuil : bureau");
        assert!(!is_compact(1280.0), "1280 px : bureau");
    }

    /// La grille rend toutes les tuiles, sans dépasser l'écran, et place
    /// effectivement deux tuiles côte à côte en fenêtre large, empilées en
    /// fenêtre étroite (mobile-first).
    #[test]
    fn grille_rend_toutes_les_tuiles_dans_l_ecran() {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx);
        let p = palette();
        for w in [360.0, 900.0, 1280.0] {
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(w, 720.0));
            ctx.begin_pass(egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            });
            // Un repère peint en fin de chaque tuile donne sa position.
            let mut marks: Vec<egui::Rect> = Vec::new();
            ctx.root_ui(|ui| {
                tile_grid(ui, &p, 340.0, 4, |ui, _p, i| {
                    card_body(ui, &p, |ui| {
                        ui.label(format!("tuile {i}"));
                        ui.add_space(8.0);
                        let (r, _) =
                            ui.allocate_exact_size(Vec2::new(10.0, 2.0), Sense::hover());
                        marks.push(r);
                    });
                });
            });
            let mut output = ctx.end_pass();
            output.textures_delta.clear();

            assert_eq!(marks.len(), 4, "4 tuiles attendues (largeur {w})");
            for r in &marks {
                assert!(
                    r.left() >= -1.0 && r.right() <= w + 1.0 && r.bottom() <= screen.height() + 1.0,
                    "tuile hors écran (largeur {w}) : {r:?}"
                );
            }
            if w >= 688.0 {
                assert!(
                    marks[1].left() >= marks[0].right() - 0.5,
                    "attendu côte à côte (largeur {w}) : {marks:?}"
                );
            } else {
                for pair in marks.windows(2) {
                    assert!(
                        pair[1].top() >= pair[0].bottom() - 0.5,
                        "attendu empilé (largeur {w}) : {marks:?}"
                    );
                }
            }
        }
    }

    /// labeled_control : libellé et contrôle sur la même ligne en largeur
    /// suffisante, empilés (libellé au-dessus) en largeur insuffisante.
    #[test]
    fn labeled_control_empile_en_fenetre_etroite() {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx);
        let p = palette();
        for (w, expect_inline) in [(600.0, true), (200.0, false)] {
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(w, 400.0));
            ctx.begin_pass(egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            });
            let mut field_top = 0.0f32;
            let start_y = ctx.root_ui(|ui| {
                let start_y = ui.cursor().top();
                labeled_control(ui, &p, "Paramètre", 400.0, |ui| {
                    let resp = ui.add(egui::TextEdit::singleline(&mut String::new()));
                    field_top = resp.rect.top();
                });
                start_y
            });
            let mut output = ctx.end_pass();
            output.textures_delta.clear();
            let delta = field_top - start_y;
            let actually_inline = delta < 10.0;
            assert_eq!(
                actually_inline, expect_inline,
                "disposition inattendue (largeur {w}, champ à +{delta:.1} px)"
            );
        }
    }

    #[test]
    fn ellipsize_coupe_avec_ellipse() {
        assert_eq!(ellipsize("abcdef", 10), "abcdef");
        assert_eq!(ellipsize("abcdef", 3), "abc…");
    }

    /// Un bouton désactivé n'accepte pas le clic et reste lisible.
    #[test]
    fn boutons_ont_une_hauteur_homogene() {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx);
        let p = palette();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(700.0, 300.0));
        ctx.begin_pass(egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        });
        let rects = ctx.root_ui(|ui| {
            [
                styled_button(ui, &p, "Principal", BtnKind::Primary).rect,
                styled_button_en(ui, &p, "Secondaire", BtnKind::Ghost, true).rect,
                styled_button_en(ui, &p, "Désactivé", BtnKind::Primary, false).rect,
                styled_button(ui, &p, "Discret", BtnKind::Quiet).rect,
            ]
        });
        let mut output = ctx.end_pass();
        output.textures_delta.clear();
        for r in rects {
            assert!((r.height() - shape::BTN_H).abs() < 0.5, "hauteur : {r:?}");
        }
    }

    /// Le texte des boutons pleins est choisi pour rester lisible : sombre sur
    /// un aplat clair (accent du thème sombre), blanc sur un aplat foncé.
    #[test]
    fn texte_des_boutons_pleins_contraste() {
        let light = Palette::new(crate::config::ThemeKind::Light);
        let dark = Palette::new(crate::config::ThemeKind::Dark);
        assert_eq!(readable_on(light.accent, &light), WHITE);
        assert_eq!(readable_on(dark.accent, &dark), dark.on_accent);
        assert_eq!(readable_on(dark.danger, &dark), dark.on_accent);
    }
}
