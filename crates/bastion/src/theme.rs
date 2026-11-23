//! Système de design : palette sobre, thème sombre/clair, typographie.
//!
//! Trois principes :
//!  - **couches** : le chrome (barres) est plus sombre que le fond de travail,
//!    lui-même plus sombre que les cartes — la hiérarchie se lit sans trait ;
//!  - **contraste** : tous les textes atteignent au moins 4,5:1 (WCAG AA) sur
//!    leur fond, y compris le texte des boutons pleins ;
//!  - **sémantique** : vert = protégé, ambre = attention, rouge = danger. Ces
//!    couleurs ne servent jamais de décoration (voir docs/ERGONOMIE.md).

use crate::config::ThemeKind;
use egui::{Color32, Context, FontId, Style, TextStyle};

/// Échelle typographique (points, avant mise à l'échelle d'accessibilité).
pub mod text {
    /// Titre d'écran.
    pub const DISPLAY: f32 = 21.0;
    /// Titre de carte.
    pub const TITLE: f32 = 14.5;
    /// Corps de texte.
    pub const BODY: f32 = 14.0;
    /// Libellés de boutons, de champs.
    pub const LABEL: f32 = 13.0;
    /// Texte secondaire, aides.
    pub const SMALL: f32 = 12.0;
    /// Métadonnées, en-têtes de colonnes.
    pub const MICRO: f32 = 11.0;
    /// Données techniques (IP, ports, commandes).
    pub const MONO: f32 = 12.5;
}

/// Échelle d'espacement (multiple de 4).
pub mod space {
    pub const XS: f32 = 4.0;
    pub const S: f32 = 8.0;
    pub const M: f32 = 12.0;
    pub const L: f32 = 16.0;
}

/// Rayons et hauteurs de contrôles, identiques partout.
pub mod shape {
    pub const R_INPUT: u8 = 8;
    pub const R_BUTTON: u8 = 9;
    pub const R_CARD: u8 = 13;
    pub const R_MODAL: u8 = 16;
    /// Hauteur d'un bouton standard (cible tactile confortable).
    pub const BTN_H: f32 = 34.0;
}

pub struct Palette {
    pub dark: bool,
    /// Fond de travail (derrière les cartes).
    pub bg: Color32,
    /// Chrome : barre latérale, barre supérieure, barre d'état.
    pub bg_alt: Color32,
    /// Cartes et surfaces posées sur le fond de travail.
    pub card: Color32,
    /// Survol des surfaces.
    pub card_hover: Color32,
    /// Fond des champs de saisie (légèrement creusé).
    pub field: Color32,
    pub border: Color32,
    /// Bordure plus marquée (séparateurs, contours au survol).
    pub border_strong: Color32,
    pub text: Color32,
    pub text_secondary: Color32,
    /// Texte posé directement sur le fond de travail (hors cartes).
    pub on_bg: Color32,
    pub on_bg_secondary: Color32,
    /// Accent : teinte terracotta de l'icône.
    pub accent: Color32,
    /// Accent au survol (plus dense).
    pub accent_hover: Color32,
    /// Texte/icône posés sur un aplat d'accent.
    pub on_accent: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub danger: Color32,
    /// Ligne alternée des tableaux.
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
    // color-scheme = 'default' → le thème GTK tranche (ex. 'Adwaita-dark').
    if let Ok(out) = std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "gtk-theme"])
        .output()
    {
        let v = String::from_utf8_lossy(&out.stdout).to_lowercase();
        if v.contains("dark") {
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
                // Papier chaud, chrome plus clair, cartes blanches : la
                // profondeur vient des valeurs, pas des ombres.
                bg: hex("#F2EDE9"),
                bg_alt: hex("#FAF7F4"),
                card: hex("#FFFFFF"),
                card_hover: hex("#F6F0EC"),
                field: hex("#FAF7F4"),
                border: hex("#E6DBD3"),
                border_strong: hex("#D3C4BA"),
                text: hex("#1F1A17"),
                text_secondary: hex("#6F625A"),
                on_bg: hex("#1F1A17"),
                on_bg_secondary: hex("#6F625A"),
                // Accent = terracotta de bastionIcon.svg, assombri pour que
                // le texte blanc dépasse 4,5:1.
                accent: hex("#B4532F"),
                accent_hover: hex("#9C4526"),
                on_accent: hex("#FFFFFF"),
                success: hex("#0F7B57"),
                warning: hex("#A15C07"),
                danger: hex("#B4233C"),
                row_alt: hex("#FAF6F3"),
            }
        } else {
            Palette {
                dark: true,
                // Neutres chauds : le terracotta reste réservé à l'accent.
                bg: hex("#141110"),
                bg_alt: hex("#0E0B0A"),
                card: hex("#1E1917"),
                card_hover: hex("#282120"),
                field: hex("#171312"),
                border: hex("#352C28"),
                border_strong: hex("#463A34"),
                text: hex("#F5EFEB"),
                text_secondary: hex("#A99A90"),
                on_bg: hex("#F5EFEB"),
                on_bg_secondary: hex("#A99A90"),
                // Terracotta clair : aplat de bouton lisible avec un texte
                // sombre (7,3:1), et teinte d'accent pour le texte (8,4:1).
                accent: hex("#E38B66"),
                accent_hover: hex("#F0A184"),
                on_accent: hex("#1B0F09"),
                success: hex("#57D9A3"),
                warning: hex("#F5C264"),
                danger: hex("#FF8095"),
                row_alt: hex("#181413"),
            }
        }
    }

    /// Fond doux dérivé d'une couleur sémantique (badges, bandeaux).
    pub fn soft(&self, c: Color32) -> Color32 {
        crate::components::blend(c, self.card, if self.dark { 0.20 } else { 0.12 })
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
    v.extreme_bg_color = p.field;
    v.code_bg_color = p.row_alt;
    v.hyperlink_color = p.accent;
    v.warn_fg_color = p.warning;
    v.error_fg_color = p.danger;
    v.window_shadow = egui::Shadow {
        offset: [0, 8],
        blur: 28,
        spread: 0,
        color: Color32::from_black_alpha(if p.dark { 120 } else { 40 }),
    };
    v.popup_shadow = egui::Shadow {
        offset: [0, 4],
        blur: 16,
        spread: 0,
        color: Color32::from_black_alpha(if p.dark { 110 } else { 32 }),
    };
    v.slider_trailing_fill = true;

    // Widgets : fond creusé pour les champs, contour discret, accent au focus.
    v.widgets.noninteractive.bg_fill = p.card;
    v.widgets.noninteractive.weak_bg_fill = p.card;
    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, p.border);
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, p.text);

    v.widgets.inactive.bg_fill = p.field;
    v.widgets.inactive.weak_bg_fill = p.field;
    v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, p.border);
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, p.text);

    v.widgets.hovered.bg_fill = p.card_hover;
    v.widgets.hovered.weak_bg_fill = p.card_hover;
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, p.border_strong);
    v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, p.text);

    v.widgets.active.bg_fill = p.accent;
    v.widgets.active.weak_bg_fill = p.accent;
    v.widgets.active.bg_stroke = egui::Stroke::new(1.0, p.accent);
    v.widgets.active.fg_stroke = egui::Stroke::new(1.0, p.on_accent);

    v.widgets.open.bg_fill = p.card_hover;
    v.widgets.open.weak_bg_fill = p.card_hover;
    v.widgets.open.bg_stroke = egui::Stroke::new(1.0, p.accent);

    v.selection.bg_fill = p.accent;
    v.selection.stroke = egui::Stroke::new(1.0, p.on_accent);

    // Coins arrondis homogènes.
    let r = egui::CornerRadius::same(shape::R_INPUT);
    v.widgets.noninteractive.corner_radius = r;
    v.widgets.inactive.corner_radius = r;
    v.widgets.hovered.corner_radius = r;
    v.widgets.active.corner_radius = r;
    v.widgets.open.corner_radius = r;
    v.menu_corner_radius = egui::CornerRadius::same(10);
    v.window_corner_radius = egui::CornerRadius::same(shape::R_MODAL);

    style.visuals = v;

    // Grille de 4 px.
    style.spacing.item_spacing = egui::vec2(space::S, space::S);
    style.spacing.button_padding = egui::vec2(12.0, 7.0);
    style.spacing.menu_margin = egui::Margin::same(10);
    style.spacing.interact_size.y = 30.0;

    // Typographie (tailles multipliées par le facteur d'accessibilité).
    let s = font_scale;
    style.text_styles = [
        (TextStyle::Heading, FontId::proportional(text::DISPLAY * s)),
        (TextStyle::Body, FontId::proportional(text::BODY * s)),
        (TextStyle::Button, FontId::proportional(text::LABEL * s)),
        (TextStyle::Monospace, FontId::monospace(text::MONO * s)),
        (TextStyle::Small, FontId::proportional(text::SMALL * s)),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Le thème actif doit être notre palette, dans les deux modes.
    #[test]
    fn theme_sombre_et_clair_bien_appliques() {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx);

        crate::theme::apply(&ctx, crate::config::ThemeKind::Dark, 1.0);
        assert_eq!(ctx.theme(), egui::Theme::Dark, "thème actif attendu : Dark");
        let st = ctx.style_of(egui::Theme::Dark);
        assert_eq!(st.visuals.panel_fill, hex("#141110"), "panel_fill sombre");
        assert_eq!(st.visuals.widgets.inactive.bg_fill, hex("#171312"), "champ sombre");

        crate::theme::apply(&ctx, crate::config::ThemeKind::Light, 1.0);
        assert_eq!(ctx.theme(), egui::Theme::Light);
        let st = ctx.style_of(egui::Theme::Light);
        assert_eq!(st.visuals.panel_fill, hex("#F2EDE9"), "panel_fill clair");

        // Une carte posée sur le fond de travail doit rester distincte.
        let p = Palette::new(crate::config::ThemeKind::Light);
        assert_ne!(p.card, p.bg, "carte et fond distincts");
        assert!(p.card.r() > p.bg.r() && p.card.g() > p.bg.g());
    }

    /// Contraste WCAG du texte sur son fond : au moins 4,5:1 partout, y
    /// compris pour le texte des boutons pleins (le point faible de la v0.2).
    #[test]
    fn contrastes_conformes_aa() {
        use crate::config::ThemeKind;
        for kind in [ThemeKind::Light, ThemeKind::Dark] {
            let p = Palette::new(kind);
            let pairs = [
                ("texte / carte", p.text, p.card),
                ("texte secondaire / carte", p.text_secondary, p.card),
                ("texte / fond", p.on_bg, p.bg),
                ("texte secondaire / fond", p.on_bg_secondary, p.bg),
                ("texte / chrome", p.text, p.bg_alt),
                ("accent / carte", p.accent, p.card),
                ("sur accent / accent", p.on_accent, p.accent),
                ("succès / carte", p.success, p.card),
                ("danger / carte", p.danger, p.card),
                ("avertissement / carte", p.warning, p.card),
            ];
            for (label, fg, bg) in pairs {
                let ratio = contraste(fg, bg);
                assert!(
                    ratio >= 4.5,
                    "{kind:?} — {label} : contraste {ratio:.2} < 4,5"
                );
            }
        }
    }

    /// Ratio de contraste WCAG (1 à 21).
    fn contraste(a: Color32, b: Color32) -> f32 {
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
        let (la, lb) = (lum(a), lum(b));
        let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
        (hi + 0.05) / (lo + 0.05)
    }
}
