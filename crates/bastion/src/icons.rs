//! Icônes Material Icons (unicolores, suivent la couleur du texte).
//! Codepoints : assets/fonts/MaterialIcons-Regular.codepoints

#![allow(dead_code)]

use egui::{FontFamily, RichText};

// Navigation
pub const LOGIN: &str = "\u{ea77}";
pub const SECURITY: &str = "\u{e32a}";
pub const BAN: &str = "\u{e166}";
pub const GPP_GOOD: &str = "\u{f013}";
pub const HISTORY: &str = "\u{e889}";
pub const SETTINGS: &str = "\u{e8b8}";

// États / sémantique
pub const SHIELD: &str = "\u{e9e0}";
pub const SHIELD_MOON: &str = "\u{eaa9}";
pub const GPP_BAD: &str = "\u{f012}";
pub const GPP_MAYBE: &str = "\u{f014}";
pub const BLOCK: &str = "\u{e14b}";
pub const WARNING: &str = "\u{e002}";
pub const ERROR: &str = "\u{e000}";
pub const INFO: &str = "\u{e88e}";
pub const CHECK: &str = "\u{e5ca}";
pub const CHECK_CIRCLE: &str = "\u{e86c}";
pub const VERIFIED_USER: &str = "\u{e8e8}";
pub const DANGEROUS: &str = "\u{e99a}";
pub const BOLT: &str = "\u{ea0b}";
pub const RULE: &str = "\u{f1c2}";
pub const POLICY: &str = "\u{ea17}";

// Actions
pub const REFRESH: &str = "\u{e5d5}";
pub const SYNC: &str = "\u{e627}";
pub const ADD: &str = "\u{e145}";
pub const DELETE: &str = "\u{e872}";
pub const RESTORE: &str = "\u{e8b3}";
pub const POWER: &str = "\u{e63c}";
pub const PLAY: &str = "\u{e037}";
pub const STOP: &str = "\u{e047}";
pub const RESTART: &str = "\u{f053}";
pub const SEARCH: &str = "\u{e8b6}";
pub const COPY: &str = "\u{e14d}";
pub const CLOSE: &str = "\u{e5cd}";
pub const EXPAND_MORE: &str = "\u{e5cf}";
pub const EXPAND_LESS: &str = "\u{e5ce}";
pub const MORE_VERT: &str = "\u{e5d4}";
pub const TUNE: &str = "\u{e429}";
pub const FILTER_ALT: &str = "\u{ef4f}";
pub const LIST: &str = "\u{e896}";
pub const OPEN_IN_NEW: &str = "\u{e89e}";
pub const SAVE: &str = "\u{e161}";
pub const DOWNLOAD: &str = "\u{f090}";
pub const UPDATE: &str = "\u{e923}";
pub const ARROW_BACK: &str = "\u{e5c4}";
pub const CHEVRON_RIGHT: &str = "\u{e5cc}";
pub const TERMINAL: &str = "\u{eb8e}";
pub const TERMINAL_SSH: &str = "\u{eb8e}";
pub const COMPUTER: &str = "\u{e30a}";
pub const DNS: &str = "\u{e875}";
pub const LOGOUT: &str = "\u{e9ba}";
pub const KEY: &str = "\u{e73c}";
pub const FINGERPRINT: &str = "\u{e90d}";
pub const FOLDER: &str = "\u{e2c7}";
pub const FOLDER_OPEN: &str = "\u{e2c8}";
pub const INBOX: &str = "\u{e156}";
pub const LOCK: &str = "\u{e897}";
pub const VISIBILITY: &str = "\u{e8f4}";
pub const VISIBILITY_OFF: &str = "\u{e8f5}";
pub const STORAGE: &str = "\u{e1db}";
pub const ARTICLE: &str = "\u{ef42}";
pub const MONITOR_HEART: &str = "\u{eaa2}";
pub const LANGUAGE: &str = "\u{e894}";
pub const TIMER: &str = "\u{e425}";
pub const SPEED: &str = "\u{e9e4}";
pub const PERSON: &str = "\u{e7fd}";
pub const VPN_KEY: &str = "\u{e0da}";
pub const NOTIFICATIONS: &str = "\u{e7f4}";
pub const CLOUD: &str = "\u{e2bd}";
pub const CLOUD_OFF: &str = "\u{e2c1}";
pub const WIFI: &str = "\u{e63e}";
pub const WIFI_OFF: &str = "\u{e648}";

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
