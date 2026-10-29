//! Système de design : palette sobre, thème sombre/clair, typographie.
//! Les couleurs sémantiques (succès/avertissement/danger) ne servent jamais
//! de décoration — voir docs/ERGONOMIE.md.

use crate::config::ThemeKind;
use egui::{Color32, Context, FontId, Style, TextStyle};

pub struct Palette {
    pub dark: bool,
    pub bg: Color32,
    pub card: Color32,
    pub card_hover: Color32,
    pub border: Color32,
    pub text: Color32,
    pub text_secondary: Color32,
    pub accent: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub danger: Color32,
    pub row_alt: Color32,
}

impl Palette {
    pub fn new(kind: ThemeKind) -> Palette {
        if kind == ThemeKind::Light {
            Palette {
                dark: false,
                bg: hex("#F4F5F7"),
                card: hex("#FFFFFF"),
                card_hover: hex("#EDEFF3"),
                border: hex("#DDE0E6"),
                text: hex("#20242A"),
                text_secondary: hex("#6B7280"),
                accent: hex("#2F6FE4"),
                success: hex("#268A52"),
                warning: hex("#B8860B"),
                danger: hex("#D33B3F"),
                row_alt: hex("#F0F2F5"),
            }
        } else {
            Palette {
                dark: true,
                bg: hex("#15161A"),
                card: hex("#1E2026"),
                card_hover: hex("#272A31"),
                border: hex("#2A2D35"),
                text: hex("#E8EAED"),
                text_secondary: hex("#9AA0AA"),
                accent: hex("#4C8DFF"),
                success: hex("#3FB96F"),
                warning: hex("#E5A13C"),
                danger: hex("#E5484D"),
                row_alt: hex("#22242B"),
            }
        }
    }
}

pub fn hex(s: &str) -> Color32 {
    let s = s.strip_prefix('#').unwrap_or(s);
    let v: u32 = u32::from_str_radix(s, 16).unwrap_or(0);
    Color32::from_rgb(
        ((v >> 16) & 0xFF) as u8,
        ((v >> 8) & 0xFF) as u8,
        (v & 0xFF) as u8,
    )
}

/// Applique le style global (palette + espacement + typographie).
/// Retourne la palette correspondante.
pub fn apply(ctx: &Context, kind: ThemeKind, font_scale: f32) -> Palette {
    let p = Palette::new(kind);

    let mut style = Style::default();
    let mut v = if p.dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    v.panel_fill = p.bg;
    v.window_fill = p.card;
    v.window_stroke = egui::Stroke::new(1.0, p.border);
    v.faint_bg_color = p.row_alt;
    v.extreme_bg_color = p.bg;
    v.code_bg_color = p.row_alt;
    v.hyperlink_color = p.accent;
    v.warn_fg_color = p.warning;
    v.error_fg_color = p.danger;

    // Zones de saisie et widgets : fond carte, bordure discrète.
    v.widgets.inactive.bg_fill = p.card;
    v.widgets.inactive.weak_bg_fill = p.card;
    v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, p.border);
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, p.text);
    v.widgets.hovered.bg_fill = p.card_hover;
    v.widgets.hovered.weak_bg_fill = p.card_hover;
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, p.accent);
    v.widgets.active.bg_fill = p.accent;
    v.widgets.active.weak_bg_fill = p.accent;
    v.widgets.active.fg_stroke = egui::Stroke::new(1.0, Color32::WHITE);
    v.widgets.open.bg_fill = p.card_hover;
    v.widgets.open.bg_stroke = egui::Stroke::new(1.0, p.accent);

    v.selection.bg_fill = p.accent;
    v.selection.stroke = egui::Stroke::new(1.0, Color32::WHITE);

    // Coins arrondis homogènes (8 px widgets, 10-12 px fenêtres/menus).
    let r = egui::CornerRadius::same(8);
    v.widgets.inactive.corner_radius = r;
    v.widgets.hovered.corner_radius = r;
    v.widgets.active.corner_radius = r;
    v.widgets.open.corner_radius = r;
    v.menu_corner_radius = egui::CornerRadius::same(10);
    v.window_corner_radius = egui::CornerRadius::same(12);

    style.visuals = v;

    // Espacement : grille de 8 px.
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 7.0);
    style.spacing.menu_margin = egui::Margin::same(10);

    // Typographie (tailles multipliées par le facteur d'accessibilité).
    let s = font_scale;
    style.text_styles = [
        (TextStyle::Heading, FontId::proportional(20.0 * s)),
        (TextStyle::Body, FontId::proportional(14.5 * s)),
        (TextStyle::Button, FontId::proportional(14.5 * s)),
        (TextStyle::Monospace, FontId::monospace(13.5 * s)),
        (TextStyle::Small, FontId::proportional(12.5 * s)),
    ]
    .into_iter()
    .collect();

    // egui 0.37 gère deux styles (sombre/clair) : on installe le nôtre
    // dans le créneau correspondant, puis on active ce créneau.
    let (theme, pref) = match kind {
        ThemeKind::Dark => (egui::Theme::Dark, egui::ThemePreference::Dark),
        ThemeKind::Light => (egui::Theme::Light, egui::ThemePreference::Light),
    };
    ctx.set_style_of(theme, std::sync::Arc::new(style));
    ctx.set_theme(pref);
    p
}
