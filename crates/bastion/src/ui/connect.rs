//! Écran Connexion : cible locale ou SSH, profils enregistrés, état de session.
//!
//! Hiérarchie : deux cartes (cette machine / SSH), un seul bouton principal
//! par carte, les options rares repliées. Les identifiants ne quittent jamais
//! la mémoire.

use crate::app::{ConnPhase, GuiApp, HostKeyCtx, Modal};
use crate::components;
use crate::config::{ProfileAuth, SshProfileDef};
use crate::theme::{space, text, Palette};
use crate::worker::Job;
use bastion_core::ssh::{SshAuth, SshParams};
use egui::{Align, CornerRadius, Frame, Layout, Margin, RichText, Stroke};
use std::path::PathBuf;

pub fn render(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;

    if app.phase == ConnPhase::Connected {
        session_summary(app, ui, p);
        return;
    }

    // Grille responsive : local et SSH côte à côte en fenêtre large,
    // empilés en fenêtre étroite (mobile-first).
    components::tile_grid(ui, p, 340.0, 2, |ui, p, i| match i {
        0 => local_card(app, ui, p),
        _ => ssh_card(app, ui, p),
    });

    ui.add_space(space::M);
    components::hint_on_bg(ui, p, t.privacy_note);
}

// ─── Session en cours ────────────────────────────────────────────────────────
fn session_summary(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    components::card_body(ui, p, |ui| {
        ui.horizontal(|ui| {
            components::status_line(ui, p, p.success, t.session_active, None);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if components::styled_button(
                    ui,
                    p,
                    &format!("{}  {}", crate::icons::LOGOUT, t.disconnect_btn),
                    components::BtnKind::Ghost,
                )
                .clicked()
                {
                    app.expect_disconnect = true;
                    app.worker.send(Job::Disconnect);
                }
            });
        });
        ui.add_space(space::M);

        egui::Grid::new("session-grid")
            .num_columns(2)
            .spacing([16.0, 8.0])
            .show(ui, |ui| {
                ui.label(RichText::new(t.target_label).size(text::SMALL).color(p.text_secondary));
                ui.label(
                    RichText::new(&app.conn_desc)
                        .monospace()
                        .family(egui::FontFamily::Name("MonoMedium".into())),
                );
                ui.end_row();
                ui.label(RichText::new(t.tools_label).size(text::SMALL).color(p.text_secondary));
                ui.horizontal(|ui| {
                    tool_badge(ui, p, "ufw", app.probe.0.as_deref(), app.tr);
                    tool_badge(ui, p, "Fail2Ban", app.probe.1.as_deref(), app.tr);
                });
                ui.end_row();
            });

        ui.add_space(space::M);
        ui.horizontal_wrapped(|ui| {
            if components::primary_button(
                ui,
                p,
                &format!("{}  {}", crate::icons::SECURITY, t.nav_firewall),
            )
            .clicked()
            {
                app.screen = crate::app::Screen::Firewall;
            }
            if components::ghost_button(ui, p, &format!("{}  {}", crate::icons::BAN, t.nav_fail2ban))
                .clicked()
            {
                app.screen = crate::app::Screen::Fail2Ban;
            }
            if components::ghost_button(ui, p, t.nav_journal).clicked() {
                app.screen = crate::app::Screen::Journal;
            }
        });
        ui.add_space(space::S);
        components::hint(ui, p, t.session_hint);
    });
}

// ─── Cette machine ───────────────────────────────────────────────────────────
fn local_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    components::card_body(ui, p, |ui| {
        card_head(ui, p, crate::icons::COMPUTER, t.local_card_title, t.badge_local);
        ui.add_space(space::S);

        ui.horizontal_wrapped(|ui| {
            tool_badge(ui, p, "ufw", app.probe.0.as_deref(), t);
            tool_badge(ui, p, "Fail2Ban", app.probe.1.as_deref(), t);
        });
        ui.add_space(space::S);

        if app.phase == ConnPhase::Connecting && !app.form.mode_ssh {
            components::loading(ui, p, t.local_wait_msg);
        } else {
            let resp = components::styled_button_fill(
                ui,
                p,
                &format!("{}  {}", crate::icons::POWER, t.connect_btn),
                components::BtnKind::Primary,
                app.phase == ConnPhase::Disconnected,
            );
            if resp.clicked() {
                app.form.mode_ssh = false;
                app.phase = ConnPhase::Connecting;
                let exe =
                    std::env::current_exe().unwrap_or_else(|_| PathBuf::from("bastion"));
                app.worker.send(Job::ConnectLocal { app_exe: exe });
            }
        }
        ui.add_space(space::S);
        components::hint(ui, p, t.local_password_hint);
    });
}

/// En-tête de carte : icône dans une pastille teintée, titre, badge à droite.
fn card_head(ui: &mut egui::Ui, p: &Palette, icon: &str, title: &str, badge: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(38.0, 38.0), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, CornerRadius::same(11), p.soft(p.accent));
        let g = ui.painter().layout_no_wrap(
            icon.to_string(),
            egui::FontId::new(20.0, egui::FontFamily::Name("Icon".into())),
            p.accent,
        );
        ui.painter()
            .galley(rect.center() - g.size() * 0.5, g, p.accent);
        ui.add_space(2.0);
        ui.label(
            RichText::new(title)
                .size(16.0)
                .color(p.text)
                .family(egui::FontFamily::Name("SemiBold".into())),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            components::badge(ui, p, badge, p.text_secondary, p.card_hover);
        });
    });
}

/// Retire le nom de l'outil en tête de la version (« ufw 0.36.2 » → « 0.36.2 »),
/// le nom étant déjà affiché dans la pastille.
fn strip_tool_name<'a>(probe: &'a str, name: &str) -> &'a str {
    let head = probe.get(..name.len()).unwrap_or_default();
    if head.eq_ignore_ascii_case(name) {
        probe[name.len()..]
            .trim_start_matches([' ', '-', ':', '/'])
            .trim()
    } else {
        probe.trim()
    }
}

/// Pastille d'outil : nom + version détectée (ou état inconnu).
fn tool_badge(
    ui: &mut egui::Ui,
    p: &Palette,
    name: &str,
    probe: Option<&str>,
    t: &crate::i18n::Tr,
) {
    let (tone, value, tip) = match probe {
        Some(v) => (
            p.success,
            components::ellipsize(strip_tool_name(v, name), 26),
            String::new(),
        ),
        None => (
            p.warning,
            t.state_unknown.to_string(),
            if name == "ufw" {
                t.probe_unknown_ufw
            } else {
                t.probe_unknown_f2b
            }
            .to_string(),
        ),
    };
    Frame::NONE
        .fill(p.soft(tone))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(9, 5))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 0.0);
                ui.label(crate::icons::icon_sz(
                    if probe.is_some() {
                        crate::icons::CHECK_CIRCLE
                    } else {
                        crate::icons::WARNING
                    },
                    13.0,
                    tone,
                ));
                let label = ui.label(
                    RichText::new(name)
                        .size(text::MICRO)
                        .color(p.text)
                        .family(egui::FontFamily::Name("Medium".into())),
                );
                if !tip.is_empty() {
                    label.on_hover_text(&tip);
                }
                ui.label(RichText::new(value).size(text::MICRO).monospace().color(p.text_secondary));
            });
        });
}

// ─── À distance (SSH) ────────────────────────────────────────────────────────
fn ssh_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    let t = app.tr;
    components::card_body(ui, p, |ui| {
        card_head(ui, p, crate::icons::TERMINAL_SSH, t.ssh_card_title, "SSH");
        ui.add_space(space::M);

        // Profils enregistrés.
        if !app.cfg.profiles.is_empty() {
            let current = app
                .form
                .profile_name
                .clone()
                .unwrap_or_else(|| t.profile_placeholder.to_string());
            ui.horizontal(|ui| {
                let del_w = if app.form.profile_name.is_some() { 34.0 } else { 0.0 };
                egui::ComboBox::from_id_salt("ssh-profile")
                    .selected_text(current)
                    .width((ui.available_width() - del_w).max(120.0))
                    .show_ui(ui, |ui| {
                        for prof in app.cfg.profiles.clone() {
                            let selected = app.form.profile_name.as_deref() == Some(&prof.name);
                            if ui
                                .selectable_label(
                                    selected,
                                    format!("{}  ·  {}@{}", prof.name, prof.user, prof.host),
                                )
                                .clicked()
                            {
                                app.form.profile_name = Some(prof.name.clone());
                                load_profile_into(app, &prof);
                            }
                        }
                    });
                if let Some(name) = app.form.profile_name.clone() {
                    if components::icon_danger_button(
                        ui,
                        p,
                        crate::icons::DELETE,
                        t.profile_delete,
                    )
                    .clicked()
                    {
                        app.cfg.delete_profile(&name);
                        app.form.profile_name = None;
                    }
                }
            });
            ui.add_space(space::S);
        }

        let form = &mut app.form;
        components::field(ui, p, t.field_host, &mut form.host, t.hint_host);
        ui.horizontal(|ui| {
            let port_w = 92.0;
            components::field_sized(ui, p, t.field_port, &mut form.port, "", port_w);
            let user_w = ui.available_width();
            components::field_sized(ui, p, t.field_user, &mut form.user, t.hint_user, user_w);
        });
        ui.add_space(space::XS);

        ui.label(
            RichText::new(t.field_auth_method)
                .size(text::SMALL)
                .color(p.text_secondary),
        );
        ui.add_space(2.0);
        let mut key = form.auth_key;
        components::segmented_fill(
            ui,
            p,
            &[(false, t.auth_password), (true, t.auth_key)],
            &mut key,
        );
        form.auth_key = key;
        ui.add_space(space::S);

        if form.auth_key {
            components::field(ui, p, t.field_key_path, &mut form.key_path, "");
            components::password_field(ui, p, t.field_passphrase, &mut form.passphrase);
        } else {
            components::password_field(ui, p, t.field_password, &mut form.password);
        }

        ui.add_space(space::M);

        let ready = {
            let f = &app.form;
            !f.host.trim().is_empty() && !f.user.trim().is_empty()
        };
        if app.phase == ConnPhase::Connecting && app.form.mode_ssh {
            components::loading(ui, p, t.status_connecting);
        } else {
            let resp = components::styled_button_fill(
                ui,
                p,
                &format!("{}  {}", crate::icons::LOGIN, t.connect_btn),
                components::BtnKind::Primary,
                ready,
            );
            if resp.clicked() {
                connect_ssh(app);
            }
        }
        ui.add_space(space::S);
        components::hint(ui, p, t.ssh_fingerprint_hint);

        // Options rares, repliées : sudo et enregistrement du profil.
        ui.add_space(space::S);
        let mut open = app.form.show_options;
        let chevron = if open {
            crate::icons::EXPAND_LESS
        } else {
            crate::icons::EXPAND_MORE
        };
        if components::link_button(ui, p, &format!("{chevron}  {}", t.ssh_options)).clicked() {
            open = !open;
            app.form.show_options = open;
        }
        if open {
            ui.add_space(space::S);
            Frame::NONE
                .fill(p.field)
                .stroke(Stroke::new(1.0, p.border))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(Margin::same(12))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let mut sudo = app.form.use_sudo;
                    if components::switch_row(
                        ui,
                        p,
                        sudo,
                        true,
                        t.use_sudo,
                        t.use_sudo_hint,
                    ) != sudo
                    {
                        sudo = !sudo;
                        app.form.use_sudo = sudo;
                    }
                    if sudo {
                        ui.add_space(space::XS);
                        components::password_field(
                            ui,
                            p,
                            t.field_sudo_password,
                            &mut app.form.sudo_password,
                        );
                    }
                    ui.add_space(space::S);
                    let ready = {
                        let f = &app.form;
                        !f.host.trim().is_empty() && !f.user.trim().is_empty()
                    };
                    ui.horizontal(|ui| {
                        let save_w = 110.0;
                        ui.add_enabled(
                            ready,
                            egui::TextEdit::singleline(&mut app.form.save_name)
                                .hint_text(t.save_profile_placeholder)
                                .desired_width((ui.available_width() - save_w - 8.0).max(120.0))
                                .margin(Margin::symmetric(10, 8)),
                        );
                        if components::styled_button_en(
                            ui,
                            p,
                            t.save_profile_btn,
                            components::BtnKind::Ghost,
                            ready,
                        )
                        .clicked()
                        {
                            save_profile(app);
                        }
                    });
                });
        }
    });
}

fn save_profile(app: &mut GuiApp) {
    let t = app.tr;
    let form = &mut app.form;
    let name = form.save_name.trim().to_string();
    if name.is_empty() {
        return;
    }
    let def = SshProfileDef {
        name,
        host: form.host.trim().to_string(),
        port: form.port.trim().parse::<u16>().unwrap_or(22),
        user: form.user.trim().to_string(),
        auth: if form.auth_key {
            ProfileAuth::Key {
                path: form.key_path.trim().to_string(),
            }
        } else {
            ProfileAuth::Password
        },
        use_sudo: form.use_sudo,
    };
    let saved_name = def.name.clone();
    app.cfg.upsert_profile(def);
    app.form.profile_name = Some(saved_name);
    app.form.save_name.clear();
    app.toasts.success(t.profile_saved_toast.to_string());
}

fn connect_ssh(app: &mut GuiApp) {
    app.form.mode_ssh = true;
    app.phase = ConnPhase::Connecting;

    let form = &mut app.form;
    let host = form.host.trim().to_string();
    let port = form.port.trim().parse::<u16>().unwrap_or(22);
    let user = form.user.trim().to_string();
    let auth = if form.auth_key {
        SshAuth::Key {
            path: PathBuf::from(form.key_path.trim()),
            passphrase: (!form.passphrase.is_empty()).then(|| form.passphrase.clone()),
        }
    } else {
        SshAuth::Password(form.password.clone())
    };
    let expected = app.cfg.known_fingerprint(&host, port);
    let params = SshParams {
        host,
        port,
        user,
        auth,
        expected_fingerprint: expected,
        use_sudo: form.use_sudo,
        sudo_password: (!form.sudo_password.trim().is_empty())
            .then(|| form.sudo_password.trim().to_string()),
    };

    // Contexte de secours pour la confirmation de clé (modale TOFU).
    let ctx = HostKeyCtx {
        host: params.host.clone(),
        fingerprint: String::new(),
        old: None,
        params: params.clone(),
    };
    app.modal = Some(Modal::HostKey(ctx));
    app.worker.send(Job::ConnectSsh { params });
}

fn load_profile_into(app: &mut GuiApp, prof: &SshProfileDef) {
    let form = &mut app.form;
    form.host = prof.host.clone();
    form.port = prof.port.to_string();
    form.user = prof.user.clone();
    form.auth_key = matches!(prof.auth, ProfileAuth::Key { .. });
    if let ProfileAuth::Key { path } = &prof.auth {
        form.key_path = path.clone();
    }
    form.use_sudo = prof.use_sudo;
}
