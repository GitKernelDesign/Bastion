//! Système de design : palette sobre, thème sombre/clair, typographie.
//! Les couleurs sémantiques (succès/avertissement/danger) ne servent jamais
//! de décoration — voir docs/ERGONOMIE.md.

use crate::config::ThemeKind;
use egui::{Color32, Context, FontId, Style, TextStyle};

pub struct Palette {
    pub dark: bool,
    pub bg: Color32,
    /// Fond des barres (latérale, haute, basse) : ton légèrement décalé
    /// pour un rendu en couches, plus moderne.
    pub bg_alt: Color32,
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

/// Détecte le thème sombre du système (GNOME / portail freedesktop / GTK_THEME).
pub fn system_dark() -> bool {
    // Schéma de couleurs GNOME 42+ : 'prefer-dark' / 'prefer-light' / 'default'.
    if let Ok(out) = std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
    {
        let v = String::from_utf8_lossy(&out.stdout);
        if v.contains("prefer-dark") {
            return true;
        }
        if v.contains("prefer-light") {
            return false;
        }
    }
    // Portail freedesktop standard : 1 = sombre, 2 = clair.
    if let Ok(out) = std::process::Command::new("gsettings")
        .args(["get", "org.freedesktop.appearance", "color-scheme"])
        .output()
    {
        if String::from_utf8_lossy(&out.stdout).trim() == "1" {
            return true;
        }
    }
    // Variable d'environnement GTK (ex. GTK_THEME=Adwaita:dark).
    if let Some(gtk) = std::env::var_os("GTK_THEME") {
        if gtk.to_string_lossy().to_lowercase().contains("dark") {
            return true;
        }
    }
    false
}

/// Résout « Système » en Dark/Light selon l'OS.
pub fn resolve(kind: ThemeKind) -> ThemeKind {
    match kind {
        ThemeKind::System => {
            if system_dark() {
                ThemeKind::Dark
            } else {
                ThemeKind::Light
            }
        }
        other => other,
    }
}

impl Palette {
    pub fn new(kind: ThemeKind) -> Palette {
        let kind = resolve(kind);
        if kind == ThemeKind::Light {
            Palette {
                dark: false,
                bg: hex("#F3F5FA"),
                bg_alt: hex("#FBFCFE"),
                card: hex("#FFFFFF"),
                card_hover: hex("#EDF0F8"),
                border: hex("#E4E7F0"),
                text: hex("#171B26"),
                text_secondary: hex("#646E84"),
                accent: hex("#4F46E5"),
                success: hex("#059669"),
                warning: hex("#D97706"),
                danger: hex("#E11D48"),
                row_alt: hex("#F1F4FA"),
            }
        } else {
            Palette {
                dark: true,
                bg: hex("#0E1016"),
                bg_alt: hex("#141824"),
                card: hex("#171C28"),
                card_hover: hex("#202636"),
                border: hex("#262D3F"),
                text: hex("#E9ECF4"),
                text_secondary: hex("#8E97AB"),
                accent: hex("#7C8CFF"),
                success: hex("#34D399"),
                warning: hex("#FBBF24"),
                danger: hex("#FB7185"),
                row_alt: hex("#141A26"),
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
    let kind = resolve(kind);
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
    v.window_corner_radius = egui::CornerRadius::same(14);

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
        _ => (egui::Theme::Light, egui::ThemePreference::Light),
    };
    ctx.set_style_of(theme, std::sync::Arc::new(style));
    ctx.set_theme(pref);
    p
}
