//! Icônes Material Icons (unicolores, suivent la couleur du texte).
//! Jeu complet : certaines constantes servent les évolutions futures.

#![allow(dead_code)]

use egui::{FontFamily, RichText};

// Codepoints : assets/fonts/MaterialIcons-Regular.codepoints
pub const SHIELD: &str = "\u{e9e0}";
pub const SECURITY: &str = "\u{e32a}";
pub const GPP_GOOD: &str = "\u{f013}";
pub const GPP_BAD: &str = "\u{f012}";
pub const GPP_MAYBE: &str = "\u{f014}";
pub const BLOCK: &str = "\u{e14b}";
pub const HISTORY: &str = "\u{e889}";
pub const SETTINGS: &str = "\u{e8b8}";
pub const REFRESH: &str = "\u{e5d5}";
pub const ADD: &str = "\u{e145}";
pub const DELETE: &str = "\u{e872}";
pub const POWER: &str = "\u{e8ac}";
pub const LOCK: &str = "\u{e897}";
pub const SEARCH: &str = "\u{e8b6}";
pub const COPY: &str = "\u{e14d}";
pub const CLOSE: &str = "\u{e5cd}";
pub const CHECK: &str = "\u{e5ca}";
pub const CHECK_CIRCLE: &str = "\u{e86c}";
pub const WARNING: &str = "\u{e002}";
pub const ERROR: &str = "\u{e000}";
pub const INFO: &str = "\u{e88e}";
pub const EXPAND_MORE: &str = "\u{e5cf}";
pub const PLAY: &str = "\u{e037}";
pub const STOP: &str = "\u{e047}";
pub const RESTART: &str = "\u{f053}";
pub const BAN: &str = "\u{e166}";
pub const COMPUTER: &str = "\u{e30a}";
pub const DNS: &str = "\u{e875}";
pub const TERMINAL: &str = "\u{eb8e}";
pub const LOGOUT: &str = "\u{e9ba}";
pub const KEY: &str = "\u{e73c}";
pub const LOGIN: &str = "\u{ea77}";
pub const LINK_OFF: &str = "\u{e16f}";
pub const STORAGE: &str = "\u{e1db}";
pub const RESTORE: &str = "\u{e8ba}";
pub const TERMINAL_SSH: &str = "\u{eb8e}";

/// Icône en taille normale (hérite la couleur du texte).
pub fn icon(code: &str) -> RichText {
    RichText::new(code).family(FontFamily::Name("Icon".into()))
}

/// Icône en taille et couleur explicites.
pub fn icon_sz(code: &str, size: f32, color: egui::Color32) -> RichText {
    RichText::new(code)
        .family(FontFamily::Name("Icon".into()))
        .size(size)
        .color(color)
}
