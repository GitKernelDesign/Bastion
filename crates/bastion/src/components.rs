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
) -> egui::Response {
    styled_button_opts(ui, p, label, kind, enabled, min_width)
}

fn styled_button_opts(
    ui: &mut Ui,
    p: &Palette,
    label: &str,
    kind: BtnKind,
    enabled: bool,
    min_width: f32,
) -> egui::Response {
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
    let mut size = galley.size() + 2.0 * pad;
    size.x = size.x.max(min_width);
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
    styled_button_opts(ui, p, label, kind, enabled, 0.0)
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

/// Aide contextuelle posée directement sur le fond de travail : en thème
/// sombre à fond clair (terracotta), la typo bascule en ton sombre.
pub fn hint_on_bg(ui: &mut Ui, p: &Palette, text: &str) {
    ui.label(RichText::new(text).size(12.5).color(p.on_bg_secondary));
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

/// En-tête d'écran (titre 20 px + sous-titre), posé sur le fond de travail.
pub fn screen_header(ui: &mut Ui, p: &Palette, title: &str, sub: &str) {
    ui.vertical(|ui| {
        ui.label(
            RichText::new(title)
                .size(20.0)
                .color(p.on_bg)
                .family(egui::FontFamily::Name("SemiBold".into())),
        );
        ui.label(RichText::new(sub).size(12.5).color(p.on_bg_secondary));
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
    let cols = tile_cols(avail, min_tile_w, tiles).min(tiles);
    let mut i = 0usize;
    while i < tiles {
        let row = cols.min(tiles - i);
        if row <= 1 {
            // Une seule tuile : pleine largeur, rendue directement.
            render(ui, p, i);
        } else {
            ui.columns(row, |cols_ui| {
                for (j, col) in cols_ui.iter_mut().enumerate() {
                    render(col, p, i + j);
                }
            });
        }
        i += row;
        if i < tiles {
            ui.add_space(TILE_GAP_Y);
        }
    }
}

/// Paramètre d'interface « libellé + contrôle » : sur une même ligne tant
/// que la largeur disponible atteint `min_inline_w`, sinon empilés
/// (libellé au-dessus, contrôle pleine largeur). C'est la brique
/// responsive de chaque paramètre des écrans.
pub fn labeled_control(
    ui: &mut Ui,
    p: &Palette,
    label: &str,
    min_inline_w: f32,
    add: impl FnOnce(&mut Ui),
) {
    let label_text = RichText::new(label)
        .size(12.5)
        .color(p.text_secondary)
        .family(egui::FontFamily::Name("Medium".into()));
    if ui.available_width() >= min_inline_w {
        ui.horizontal(|ui| {
            ui.label(label_text);
            add(ui);
        });
    } else {
        ui.label(label_text);
        add(ui);
    }
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
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, 720.0));
            ctx.begin_pass(egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            });
            // Un repère peint en fin de chaque tuile donne sa position.
            let mut marks: Vec<egui::Rect> = Vec::new();
            ctx.root_ui(|ui| {
                tile_grid(ui, &p, 340.0, 4, |ui, _p, i| {
                    card_body(ui, &p, |ui| {
                        ui.set_min_width(ui.available_width() - 2.0);
                        ui.label(format!("tuile {i}"));
                        ui.add_space(8.0);
                        let (r, _) = ui.allocate_exact_size(
                            egui::vec2(10.0, 2.0),
                            egui::Sense::hover(),
                        );
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
                // Deux tuiles côte à côte : la 2e commence après la 1re.
                assert!(
                    marks[1].left() >= marks[0].right() - 0.5,
                    "attendu côte à côte (largeur {w}) : {marks:?}"
                );
            } else {
                // Empilées : chaque tuile commence sous la précédente.
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
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, 400.0));
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
            // En ligne : le champ démarre quasiment au même niveau que le
            // libellé. Empilé : il démarre sous la ligne du libellé (~20 px).
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
}
