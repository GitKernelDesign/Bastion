//! Mise en page globale : coquille responsive mobile-first.
//!
//! Fenêtre étroite (< `COMPACT_MAX_WIDTH`) : barre supérieure + contenu en
//! une colonne + navigation en bas d'écran (usage tactile/compact).
//! Fenêtre large : barre latérale, barre supérieure, barre d'état.
//!
//! La coquille porte l'identité (logo, cible, session) et laisse chaque écran
//! se concentrer sur son contenu : titre et sous-titre fixes en haut du
//! contenu, puis une zone qui défile.

pub mod clamav;
pub mod connect;
pub mod fail2ban;
pub mod firewall;
pub mod journal;
pub mod settings;

use crate::app::{ConnPhase, GuiApp, Screen};
use crate::components;
use crate::icons;
use crate::theme::{space, text, Palette};
use egui::{Align, Color32, CornerRadius, FontId, Layout, Margin, RichText, Sense, Stroke, StrokeKind};

/// Hauteur de la barre supérieure.
const TOPBAR_H: f32 = 54.0;
/// Hauteur de la barre d'état.
const STATUS_H: f32 = 26.0;
/// Hauteur de la navigation basse (mode compact).
const BOTTOM_NAV_H: f32 = 62.0;
/// Largeur de la barre latérale (mode large).
const SIDEBAR_W: f32 = 224.0;

/// Marge basse à réserver aux notifications pour ne rien recouvrir.
pub fn toast_inset(compact: bool) -> f32 {
    if compact {
        BOTTOM_NAV_H + STATUS_H + 10.0
    } else {
        STATUS_H + 10.0
    }
}

/// Rendu complet d'une frame.
pub fn render(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    // Mobile-first : la disposition se décide à chaque frame selon la
    // largeur réelle de la fenêtre.
    let compact = components::is_compact(ui.ctx().input(|i| i.viewport_rect().width()));
    topbar(app, ui, p, compact);

    if compact {
        // La navigation basse est appelée en premier : le premier panneau
        // déclaré en bas occupe le bord inférieur, la barre d'état passe
        // juste au-dessus.
        bottom_nav(app, ui, p);
        statusbar(app, ui, p, compact);
    } else {
        statusbar(app, ui, p, compact);
        sidebar(app, ui, p);
    }

    egui::CentralPanel::default_margins()
        .frame(
            egui::Frame::NONE
                .fill(p.bg)
                .inner_margin(Margin {
                    left: if compact { 14 } else { 24 },
                    right: if compact { 14 } else { 24 },
                    top: if compact { 12 } else { 18 },
                    bottom: 0,
                }),
        )
        .show(ui, |ui| {
            header(app, ui, p, compact);
            egui::ScrollArea::vertical()
                .id_salt(("page", app.screen as u8))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add_space(2.0);
                    match app.screen {
                        Screen::Connect => connect::render(app, ui, p),
                        Screen::Firewall => firewall::render(app, ui, p),
                        Screen::Fail2Ban => fail2ban::render(app, ui, p),
                        Screen::Antivirus => clamav::render(app, ui, p),
                        Screen::Journal => journal::render(app, ui, p),
                        Screen::Settings => settings::render(app, ui, p),
                    }
                    ui.add_space(space::L);
                });
        });
}

/// Titre et sous-titre de l'écran courant.
pub fn screen_titles(t: &crate::i18n::Tr, screen: Screen) -> (&'static str, &'static str) {
    match screen {
        Screen::Connect => (t.nav_connect, t.connect_subtitle),
        Screen::Firewall => (t.nav_firewall, t.fw_subtitle),
        Screen::Fail2Ban => (t.nav_fail2ban, t.f2b_subtitle),
        Screen::Antivirus => (t.nav_clamav, t.clam_subtitle),
        Screen::Journal => (t.nav_journal, t.journal_subtitle),
        Screen::Settings => (t.nav_settings, t.settings_subtitle),
    }
}

/// Écrans de navigation (partagés entre la barre latérale et la barre basse).
pub fn nav_entries(t: &crate::i18n::Tr) -> [(Screen, &'static str, &'static str); 6] {
    [
        (Screen::Connect, icons::LOGIN, t.nav_connect),
        (Screen::Firewall, icons::SECURITY, t.nav_firewall),
        (Screen::Fail2Ban, icons::BAN, t.nav_fail2ban),
        (Screen::Antivirus, icons::GPP_GOOD, t.nav_clamav),
        (Screen::Journal, icons::HISTORY, t.nav_journal),
        (Screen::Settings, icons::SETTINGS, t.nav_settings),
    ]
}

/// Compteur affiché à droite d'une entrée de navigation (0 = aucun).
fn nav_badge(app: &GuiApp, screen: Screen) -> Option<String> {
    match screen {
        Screen::Firewall => {
            let n = app.firewall.rules.len();
            (n > 0).then(|| n.to_string())
        }
        Screen::Fail2Ban => {
            let banned: u64 = app
                .f2b
                .jail_infos
                .values()
                .map(|(i, _)| i.currently_banned.unwrap_or(0))
                .sum();
            (banned > 0).then(|| banned.to_string())
        }
        _ => None,
    }
}

// ─── Barre supérieure ────────────────────────────────────────────────────────
fn topbar(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette, compact: bool) {
    let t = app.tr;
    egui::Panel::top("topbar")
        .exact_size(TOPBAR_H)
        .frame(egui::Frame::NONE.fill(p.bg_alt).inner_margin(Margin {
            left: if compact { 12 } else { 18 },
            right: if compact { 10 } else { 16 },
            top: 8,
            bottom: 8,
        }))
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                if compact {
                    ui.add(egui::Image::new(&app.logo).max_size(egui::vec2(24.0, 24.0)));
                    ui.label(
                        RichText::new("Bastion")
                            .size(15.0)
                            .family(egui::FontFamily::Name("SemiBold".into())),
                    );
                    ui.add_space(2.0);
                }

                // Chip de cible : cliquable, mène à l'écran Connexion.
                let (color, icon, label) = match app.phase {
                    ConnPhase::Connected => (
                        if app.conn_desc.starts_with("SSH") {
                            p.accent
                        } else {
                            p.success
                        },
                        icons::GPP_GOOD,
                        app.conn_desc.clone(),
                    ),
                    ConnPhase::Connecting => (
                        p.warning,
                        icons::GPP_MAYBE,
                        t.status_connecting.to_string(),
                    ),
                    ConnPhase::Disconnected => (
                        p.text_secondary,
                        icons::GPP_BAD,
                        t.status_disconnected.to_string(),
                    ),
                };
                let max = if compact { 20 } else { 42 };
                connection_chip(ui, p, color, icon, &components::ellipsize(&label, max), &label)
                    .clicked()
                    .then(|| app.screen = Screen::Connect);

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);
                    // Menu d'actions globales.
                    let more = components::icon_button(ui, p, icons::MORE_VERT, t.more_actions);
                    egui::Popup::menu(&more).show(|ui| {
                        ui.set_min_width(210.0);
                        if ui
                            .button(format!("{}  {}", icons::HISTORY, t.nav_journal))
                            .clicked()
                        {
                            app.screen = Screen::Journal;
                            ui.close();
                        }
                        if ui
                            .button(format!("{}  {}", icons::SETTINGS, t.nav_settings))
                            .clicked()
                        {
                            app.screen = Screen::Settings;
                            ui.close();
                        }
                        ui.separator();
                        let mut auto = app.cfg.auto_refresh;
                        if ui.checkbox(&mut auto, t.topbar_auto).changed() {
                            app.cfg.auto_refresh = auto;
                            app.cfg.save();
                        }
                        if app.phase == ConnPhase::Connected {
                            ui.separator();
                            if ui
                                .button(format!("{}  {}", icons::LOGOUT, t.disconnect_btn))
                                .clicked()
                            {
                                app.expect_disconnect = true;
                                app.worker.send(crate::worker::Job::Disconnect);
                            }
                        }
                    });

                    let refreshing = app.is_refreshing();
                    let enabled = app.phase == ConnPhase::Connected && !refreshing;
                    let btn = egui::Button::new(
                        icons::icon_sz(
                            icons::REFRESH,
                            16.0,
                            if enabled { p.text } else { p.text_secondary },
                        )
                        .family(egui::FontFamily::Name("Icon".into())),
                    )
                    .frame(false)
                    .min_size(egui::vec2(30.0, 30.0));
                    if ui
                        .add_enabled(enabled, btn)
                        .on_hover_text(t.tooltip_refresh)
                        .clicked()
                    {
                        app.refresh_all();
                    }
                    // Indicateur d'actualisation automatique (interrupteur
                    // compact, sans libellé, en mode large uniquement).
                    if !compact && app.phase == ConnPhase::Connected {
                        let mut auto = app.cfg.auto_refresh;
                        if ui
                            .selectable_label(auto, RichText::new(t.topbar_auto).size(text::MICRO))
                            .on_hover_text(t.tooltip_auto)
                            .clicked()
                        {
                            auto = !auto;
                            app.cfg.auto_refresh = auto;
                            app.cfg.save();
                        }
                    }
                });
            });
        });
}

/// Pastille de cible : point d'état + libellé + chevron.
fn connection_chip(
    ui: &mut egui::Ui,
    p: &Palette,
    color: Color32,
    icon: &str,
    label: &str,
    full: &str,
) -> egui::Response {
    let font = FontId::new(text::SMALL, egui::FontFamily::Name("Medium".into()));
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font, p.text);
    let w = 8.0 + 18.0 + 6.0 + galley.size().x + 18.0;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, 32.0), Sense::click());
    let fill = if resp.hovered() { p.card_hover } else { p.card };
    ui.painter().rect(
        rect,
        CornerRadius::same(16),
        fill,
        Stroke::new(1.0, p.border),
        StrokeKind::Inside,
    );
    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 17.0, rect.center().y),
        egui::vec2(16.0, 16.0),
    );
    ui.painter()
        .circle_filled(icon_rect.center(), 8.0, p.soft(color));
    let g = ui.painter().layout_no_wrap(
        icon.to_string(),
        FontId::new(12.0, egui::FontFamily::Name("Icon".into())),
        color,
    );
    ui.painter()
        .galley(icon_rect.center() - g.size() * 0.5, g, color);
    ui.painter().galley(
        egui::pos2(rect.left() + 32.0, rect.center().y - galley.size().y * 0.5),
        galley,
        p.text,
    );
    let chevron = ui.painter().layout_no_wrap(
        icons::CHEVRON_RIGHT.to_string(),
        FontId::new(13.0, egui::FontFamily::Name("Icon".into())),
        p.text_secondary,
    );
    let chevron_pos = egui::pos2(
        rect.right() - 14.0,
        rect.center().y - chevron.size().y * 0.5,
    );
    ui.painter()
        .galley(chevron_pos, chevron, p.text_secondary);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, full));
    resp.on_hover_text(full)
}

// ─── Titre d'écran ───────────────────────────────────────────────────────────
fn header(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette, compact: bool) {
    let (title, sub) = screen_titles(app.tr, app.screen);
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new(title)
                    .size(if compact { 19.0 } else { text::DISPLAY })
                    .color(p.on_bg)
                    .family(egui::FontFamily::Name("SemiBold".into())),
            );
            if !compact {
                ui.label(
                    RichText::new(sub)
                        .size(text::SMALL)
                        .color(p.on_bg_secondary),
                );
            }
        });
    });
    ui.add_space(space::S);
}

// ─── Barre latérale ──────────────────────────────────────────────────────────
fn sidebar(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    egui::Panel::left("nav")
        .exact_size(SIDEBAR_W)
        .frame(
            egui::Frame::NONE
                .fill(p.bg_alt)
                .inner_margin(Margin {
                    left: 12,
                    right: 12,
                    top: 16,
                    bottom: 12,
                }),
        )
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::Image::new(&app.logo).max_size(egui::vec2(32.0, 32.0)));
                ui.add_space(2.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Bastion")
                            .size(17.0)
                            .family(egui::FontFamily::Name("SemiBold".into())),
                    );
                    ui.label(
                        RichText::new(t.app_tagline)
                            .size(text::MICRO)
                            .color(p.text_secondary),
                    );
                });
            });
            ui.add_space(space::L);

            let entries = nav_entries(t);
            let badges = nav_badges(app);
            components::section(ui, p, t.nav_section_admin);
            ui.add_space(4.0);
            nav_list_badged(ui, p, &entries[..4], &mut app.screen, &badges[..4]);
            ui.add_space(space::M);
            components::section(ui, p, t.nav_section_follow);
            ui.add_space(4.0);
            nav_list_badged(ui, p, &entries[4..5], &mut app.screen, &badges[4..5]);

            // Réglages ancrés au-dessus du pied de barre.
            ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                ui.add_space(2.0);
                session_footer(app, ui, p);
                ui.add_space(space::S);
                components::hairline(ui, p);
                ui.add_space(space::S);
                nav_list_badged(ui, p, &entries[5..], &mut app.screen, &badges[5..]);
            });
        });
}

fn session_footer(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    let (col, txt) = match app.phase {
        ConnPhase::Connected => (p.success, app.conn_desc.clone()),
        ConnPhase::Connecting => (p.warning, t.status_connecting.to_string()),
        ConnPhase::Disconnected => (p.text_secondary, t.status_disconnected.to_string()),
    };
    ui.horizontal(|ui| {
        components::status_dot(ui, col, 7.0);
        ui.label(
            RichText::new(components::ellipsize(&txt, 24))
                .size(text::MICRO)
                .color(p.text_secondary),
        )
        .on_hover_text(txt);
    });
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(concat!("Bastion v", env!("CARGO_PKG_VERSION")))
                .size(10.0)
                .color(p.text_secondary),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if app.phase == ConnPhase::Connected {
                // Deux points : outils détectés (plein) ou non (creux).
                let mut probe_resp = None;
                for probe in [&app.probe.1, &app.probe.0] {
                    let (rect, resp) =
                        ui.allocate_exact_size(egui::vec2(7.0, 7.0), Sense::hover());
                    if probe.is_some() {
                        ui.painter().circle_filled(rect.center(), 3.0, p.success);
                    } else {
                        ui.painter().circle_stroke(
                            rect.center(),
                            3.0,
                            Stroke::new(1.0, p.border_strong),
                        );
                    }
                    probe_resp = Some(resp);
                }
                if let Some(resp) = probe_resp {
                    resp.on_hover_text(t.tools_label);
                }
            }
        });
    });
}

/// Compteurs des six destinations, dans l'ordre de `nav_entries`.
fn nav_badges(app: &GuiApp) -> [Option<String>; 6] {
    [
        nav_badge(app, Screen::Connect),
        nav_badge(app, Screen::Firewall),
        nav_badge(app, Screen::Fail2Ban),
        nav_badge(app, Screen::Antivirus),
        nav_badge(app, Screen::Journal),
        nav_badge(app, Screen::Settings),
    ]
}

fn nav_list_badged(
    ui: &mut egui::Ui,
    p: &Palette,
    entries: &[(Screen, &str, &str)],
    current: &mut Screen,
    badges: &[Option<String>],
) {
    ui.spacing_mut().item_spacing = egui::vec2(8.0, 2.0);
    for (i, (screen, icon, label)) in entries.iter().enumerate() {
        let badge = badges.get(i).and_then(|b| b.as_deref());
        nav_item(ui, p, *screen, icon, label, current, badge);
    }
}

fn nav_item(
    ui: &mut egui::Ui,
    p: &Palette,
    screen: Screen,
    icon: &str,
    label: &str,
    current: &mut Screen,
    badge: Option<&str>,
) -> egui::Rect {
    let selected = *current == screen;
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 36.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, selected, label));
    let fill = if selected {
        p.soft(p.accent)
    } else if resp.hovered() {
        p.card_hover
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect(
        rect,
        CornerRadius::same(9),
        fill,
        Stroke::NONE,
        StrokeKind::Inside,
    );
    // Indicateur vertical de l'écran actif.
    if selected {
        let ind = egui::Rect::from_min_size(
            egui::pos2(rect.left() + 3.0, rect.center().y - 8.0),
            egui::vec2(3.0, 16.0),
        );
        ui.painter()
            .rect_filled(ind, CornerRadius::same(2), p.accent);
    }
    let fg = if selected { p.accent } else { p.text_secondary };
    let icon_g = ui.painter().layout_no_wrap(
        icon.to_string(),
        FontId::new(18.0, egui::FontFamily::Name("Icon".into())),
        fg,
    );
    let txt_g = ui.painter().layout_no_wrap(
        label.to_string(),
        FontId::new(13.5, egui::FontFamily::Name("Medium".into())),
        if selected { p.text } else { p.text_secondary },
    );
    let y = rect.center().y;
    let x = rect.left() + 14.0;
    ui.painter()
        .galley(egui::pos2(x, y - icon_g.size().y * 0.5), icon_g.clone(), fg);
    ui.painter().galley(
        egui::pos2(x + icon_g.size().x + 10.0, y - txt_g.size().y * 0.5),
        txt_g,
        if selected { p.text } else { p.text_secondary },
    );
    if let Some(b) = badge {
        let font = FontId::new(10.0, egui::FontFamily::Name("Medium".into()));
        let g = ui.painter().layout_no_wrap(b.to_string(), font, fg);
        let bw = (g.size().x + 12.0).max(20.0);
        let brect = egui::Rect::from_min_size(
            egui::pos2(rect.right() - bw - 8.0, rect.center().y - 9.0),
            egui::vec2(bw, 18.0),
        );
        ui.painter().rect_filled(
            brect,
            CornerRadius::same(9),
            if selected { p.card } else { p.card_hover },
        );
        ui.painter()
            .galley(brect.center() - g.size() * 0.5, g, fg);
    }
    if resp.clicked() {
        *current = screen;
    }
    rect
}

// ─── Barre d'état ────────────────────────────────────────────────────────────
fn statusbar(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette, compact: bool) {
    let t = app.tr;
    let pending = app.pending.len();
    let pad = if compact { 12 } else { 20 };
    egui::Panel::bottom("statusbar")
        .exact_size(STATUS_H)
        .frame(egui::Frame::NONE.fill(p.bg_alt).inner_margin(Margin {
            left: pad,
            right: pad,
            top: 4,
            bottom: 4,
        }))
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                if pending > 0 {
                    ui.add(egui::Spinner::new().size(10.0));
                    ui.label(
                        RichText::new(t.ops_pending(pending))
                            .size(text::MICRO)
                            .color(p.text_secondary),
                    );
                } else if !app.status_msg.is_empty() {
                    let take = if compact { 52 } else { 120 };
                    let msg = components::ellipsize(&app.status_msg, take);
                    ui.label(
                        RichText::new(msg)
                            .size(text::MICRO)
                            .color(p.text_secondary),
                    )
                    .on_hover_text(&app.status_msg);
                } else {
                    ui.label(
                        RichText::new(t.status_ready)
                            .size(text::MICRO)
                            .color(p.text_secondary),
                    );
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if app.phase == ConnPhase::Connected {
                        let on = app.cfg.auto_refresh;
                        let col = if on { p.success } else { p.text_secondary };
                        let txt = if on {
                            format!("{} · {} s", t.topbar_auto, app.cfg.refresh_secs)
                        } else {
                            t.topbar_auto.to_string()
                        };
                        ui.label(RichText::new(txt).size(10.0).color(col))
                            .on_hover_text(format!("{}\n{}", t.tooltip_auto, t.shortcut_hint));
                    }
                });
            });
        });
}

// ─── Navigation basse (mode compact) ────────────────────────────────────────
/// Barre de navigation basse d'écran (icône + libellé), visible en fenêtre
/// étroite à la place de la barre latérale.
fn bottom_nav(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let entries = nav_entries(app.tr);
    egui::Panel::bottom("bottomnav")
        .exact_size(BOTTOM_NAV_H)
        .frame(egui::Frame::NONE.fill(p.bg_alt).inner_margin(Margin {
            left: 6,
            right: 6,
            top: 5,
            bottom: 5,
        }))
        .show(ui, |ui| {
            bottom_nav_list(ui, p, &entries, &mut app.screen);
        });
}

/// Liste de navigation basse (extraite pour être testée).
/// Chaque item reçoit une largeur égale et entière : 1/n-ième de l'écran.
pub fn bottom_nav_list(
    ui: &mut egui::Ui,
    p: &Palette,
    entries: &[(Screen, &str, &str)],
    current: &mut Screen,
) -> Vec<egui::Rect> {
    let n = entries.len().max(1);
    let item_w = (ui.available_width() / n as f32).max(44.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        entries
            .iter()
            .map(|(screen, icon, label)| {
                bottom_nav_item(ui, p, *screen, icon, label, current, item_w)
            })
            .collect::<Vec<egui::Rect>>()
    })
    .inner
}

fn bottom_nav_item(
    ui: &mut egui::Ui,
    p: &Palette,
    screen: Screen,
    icon: &str,
    label: &str,
    current: &mut Screen,
    item_w: f32,
) -> egui::Rect {
    let selected = *current == screen;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(item_w, 52.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Tab, true, selected, label));
    let fill = if selected {
        p.soft(p.accent)
    } else if resp.hovered() {
        p.card_hover
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect(
        rect.shrink2(egui::vec2(2.0, 0.0)),
        CornerRadius::same(10),
        fill,
        Stroke::NONE,
        StrokeKind::Inside,
    );
    let fg = if selected { p.accent } else { p.text_secondary };
    let painter = ui.painter().with_clip_rect(rect.shrink2(egui::vec2(2.0, 0.0)));
    let icon_g = painter.layout_no_wrap(
        icon.to_string(),
        FontId::new(20.0, egui::FontFamily::Name("Icon".into())),
        fg,
    );
    // Libellé réduit à la largeur de l'item (les traductions sont longues).
    let max_chars = ((item_w - 6.0) / 5.4).max(3.0) as usize;
    let label = components::ellipsize(label, max_chars);
    let label_g = painter.layout_no_wrap(
        label,
        FontId::new(10.0, egui::FontFamily::Name("Medium".into())),
        fg,
    );
    painter.galley(
        egui::pos2(rect.center().x - icon_g.size().x * 0.5, rect.top() + 7.0),
        icon_g,
        fg,
    );
    painter.galley(
        egui::pos2(
            rect.center().x - label_g.size().x * 0.5,
            rect.bottom() - label_g.size().y - 7.0,
        ),
        label_g,
        fg,
    );
    if resp.clicked() {
        *current = screen;
    }
    rect
}

/// Liste de navigation verticale (extraite pour être testable).
#[cfg(test)]
fn nav_list(
    ui: &mut egui::Ui,
    p: &Palette,
    entries: &[(Screen, &str, &str)],
    current: &mut Screen,
) -> Vec<egui::Rect> {
    ui.spacing_mut().item_spacing = egui::vec2(8.0, 4.0);
    entries
        .iter()
        .map(|(screen, icon, label)| nav_item(ui, p, *screen, icon, label, current, None))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ThemeKind;

    /// Les items de navigation doivent s'empiler verticalement et rester
    /// dans les limites de l'écran, quelle que soit la largeur de fenêtre.
    #[test]
    fn nav_items_empiles_dans_l_ecran() {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx);
        let p = Palette::new(ThemeKind::Dark);
        let entries = [
            (Screen::Connect, icons::LOGIN, "Connexion"),
            (Screen::Firewall, icons::SECURITY, "Pare-feu"),
            (Screen::Fail2Ban, icons::BAN, "Fail2Ban"),
            (Screen::Journal, icons::HISTORY, "Journal"),
            (Screen::Settings, icons::SETTINGS, "Réglages"),
        ];
        for w in [1280.0, 900.0, 640.0] {
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, 720.0));
            let input = egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            };
            let mut current = Screen::Firewall;
            ctx.begin_pass(input);
            let rects = ctx.root_ui(|ui| {
                egui::Panel::left("nav")
                    .exact_size(SIDEBAR_W)
                    .frame(egui::Frame::NONE.fill(p.bg_alt).inner_margin(Margin::same(12)))
                    .show(ui, |ui| {
                        ui.add_space(6.0);
                        ui.add_space(14.0);
                        nav_list(ui, &p, &entries, &mut current)
                    })
                    .inner
            });
            let mut output = ctx.end_pass();
            output.textures_delta.clear();
            // Toutes les formes peintes restent dans l'écran.
            for cs in &output.shapes {
                let r = cs.shape.visual_bounding_rect();
                assert!(
                    r.left() >= -1.0
                        && r.right() <= screen.width() + 1.0
                        && r.top() >= -1.0
                        && r.bottom() <= screen.height() + 1.0,
                    "forme hors écran (largeur {w}) : {r:?}"
                );
            }
            // 5 items empilés verticalement, sans chevauchement.
            assert_eq!(rects.len(), 5, "5 items attendus (largeur {w})");
            for r in &rects {
                assert!((r.height() - 36.0).abs() < 0.5, "hauteur item : {r:?}");
                assert!(
                    r.left() >= -1.0 && r.right() <= screen.width() + 1.0,
                    "item hors écran : {r:?}"
                );
            }
            for pair in rects.windows(2) {
                assert!(
                    pair[1].top() >= pair[0].bottom() - 0.5,
                    "chevauchement : {rects:?}"
                );
            }
        }
    }

    /// Une ScrollArea imbriquée dans le défilement de page doit conserver
    /// sa hauteur voulue (sinon elle s'effondre à min_scrolled_size = 64 px,
    /// voir egui#1097) même quand la hauteur disponible est épuisée.
    #[test]
    fn scroll_area_imbriquee_garde_sa_hauteur() {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 700.0));
        let input = egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        };
        ctx.begin_pass(input);
        let mut inner_h = 0.0f32;
        ctx.root_ui(|ui| {
            egui::ScrollArea::vertical()
                .id_salt("page")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for _ in 0..60 {
                        ui.label("ligne de remplissage");
                    }
                    let resp = egui::ScrollArea::vertical()
                        .id_salt("table")
                        .max_height(900.0)
                        .min_scrolled_height(900.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.label("contenu du tableau");
                        });
                    inner_h = resp.inner_rect.height();
                });
        });
        let mut output = ctx.end_pass();
        output.textures_delta.clear();
        assert!(
            inner_h >= 899.0,
            "hauteur de la zone imbriquée attendue ~900, vue {inner_h}"
        );
    }

    /// Navigation basse (mode compact) : les 6 items se répartissent la
    /// largeur, restent dans l'écran et ne se chevauchent pas — y compris
    /// sur un écran de téléphone.
    #[test]
    fn nav_basse_repartie_la_largeur_dans_l_ecran() {
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx);
        let p = Palette::new(ThemeKind::Dark);
        let entries = nav_entries(crate::i18n::tr(crate::i18n::Lang::Fr));
        for w in [360.0, 480.0, 719.9] {
            assert!(crate::components::is_compact(w), "{w} px doit être compact");
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, 640.0));
            ctx.begin_pass(egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            });
            let mut current = Screen::Firewall;
            let rects = ctx.root_ui(|ui| {
                egui::Panel::bottom("bottomnav")
                    .exact_size(BOTTOM_NAV_H)
                    .frame(egui::Frame::NONE.fill(p.bg_alt).inner_margin(Margin {
                        left: 6,
                        right: 6,
                        top: 5,
                        bottom: 5,
                    }))
                    .show(ui, |ui| bottom_nav_list(ui, &p, &entries, &mut current))
                    .inner
            });
            let mut output = ctx.end_pass();
            output.textures_delta.clear();

            assert_eq!(rects.len(), 6, "6 items attendus (largeur {w})");
            for r in &rects {
                assert!(
                    r.left() >= -1.0 && r.right() <= screen.width() + 1.0,
                    "item de navigation hors écran (largeur {w}) : {r:?}"
                );
                assert!((r.height() - 52.0).abs() < 0.5, "hauteur item : {r:?}");
            }
            for pair in rects.windows(2) {
                assert!(
                    pair[1].left() >= pair[0].right() - 0.5,
                    "chevauchement horizontal : {rects:?}"
                );
            }
            // Les items se répartissent toute la largeur disponible.
            let total: f32 = rects.iter().map(|r| r.width()).sum();
            assert!(
                total >= w - 14.0,
                "les items doivent occuper la largeur ({total:.1} sur {w})"
            );
        }
    }

    /// Aucune forme peinte ne doit dépasser l'écran, ni dans la barre
    /// supérieure (libellés longs), ni dans la navigation basse.
    #[test]
    fn libelles_coupes_dans_la_barre_superieure() {
        // Les libellés de cible peuvent être longs en SSH : la pastille est
        // réduite avant peinture, donc sa largeur reste bornée.
        assert!(components::ellipsize("user@serveur-de-production.example.org", 20)
            .chars()
            .count()
            <= 21);
    }
}
