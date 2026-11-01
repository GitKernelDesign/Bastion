//! Écran Connexion : cible locale ou SSH, profils enregistrés, état de session.

use crate::app::{ConnPhase, GuiApp, HostKeyCtx, Modal};
use crate::components;
use crate::config::{ProfileAuth, SshProfileDef};
use crate::worker::Job;
use egui::{Align, Layout, RichText};
use bastion_core::ssh::{SshAuth, SshParams};
use std::path::PathBuf;

pub fn render(app: &mut GuiApp, ui: &mut egui::Ui, p: &crate::theme::Palette) {
    let t = app.tr;
    components::screen_header(ui, p, t.nav_connect, t.connect_subtitle);
    ui.add_space(6.0);

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

    ui.add_space(10.0);
    components::hint_on_bg(ui, p, t.privacy_note);
}

fn session_summary(app: &mut GuiApp, ui: &mut egui::Ui, p: &crate::theme::Palette) {
    let t = app.tr;
    components::card_body(ui, p, |ui| {
        ui.horizontal(|ui| {
            components::status_line(ui, p, p.success, t.session_active, None);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if components::danger_button(ui, p, t.disconnect_btn).clicked() {
                    app.expect_disconnect = true;
                    app.worker.send(Job::Disconnect);
                }
            });
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(t.target_label).size(12.5).color(p.text_secondary));
            ui.label(
                RichText::new(&app.conn_desc)
                    .monospace()
                    .family(egui::FontFamily::Name("MonoMedium".into())),
            );
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new(t.tools_label).size(12.5).color(p.text_secondary));
            ui.label(probe_ufw_text(app));
            ui.label("·");
            ui.label(probe_f2b_text(app));
        });
        ui.add_space(4.0);
        components::hint(ui, p, t.session_hint);
    });
}

fn probe_ufw_text(app: &GuiApp) -> String {
    app.probe
        .0
        .clone()
        .unwrap_or_else(|| app.tr.probe_unknown_ufw.to_string())
}

fn probe_f2b_text(app: &GuiApp) -> String {
    app.probe
        .1
        .clone()
        .unwrap_or_else(|| app.tr.probe_unknown_f2b.to_string())
}

fn local_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &crate::theme::Palette) {
    let t = app.tr;
    components::card_body(ui, p, |ui| {
        ui.horizontal(|ui| {
            ui.label(crate::icons::icon_sz(crate::icons::COMPUTER, 20.0, p.accent));
            ui.label(
                RichText::new(t.local_card_title)
                    .size(15.0)
                    .family(egui::FontFamily::Name("SemiBold".into())),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                components::badge(ui, p, t.badge_local, p.text_secondary, p.card_hover);
            });
        });
        ui.add_space(8.0);
        components::hint(ui, p, &format!("{}\n{}", probe_ufw_text(app), probe_f2b_text(app)));
        ui.add_space(10.0);

        if app.phase == ConnPhase::Connecting && !app.form.mode_ssh {
            components::loading(ui, p, t.local_wait_msg);
        } else if components::primary_button(ui, p, t.connect_btn).clicked() {
            app.form.mode_ssh = false;
            app.phase = ConnPhase::Connecting;
            let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("bastion"));
            app.worker.send(Job::ConnectLocal { app_exe: exe });
        }
        ui.add_space(2.0);
        components::hint(ui, p, t.local_password_hint);
    });
}

fn ssh_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &crate::theme::Palette) {
    let t = app.tr;
    components::card_body(ui, p, |ui| {
        ui.horizontal(|ui| {
            ui.label(crate::icons::icon_sz(crate::icons::TERMINAL_SSH, 20.0, p.accent));
            ui.label(
                RichText::new(t.ssh_card_title)
                    .size(15.0)
                    .family(egui::FontFamily::Name("SemiBold".into())),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                components::badge(ui, p, "SSH", p.accent, p.card_hover);
            });
        });
        ui.add_space(8.0);

        // Profil enregistré.
        if !app.cfg.profiles.is_empty() {
            let current = app
                .form
                .profile_name
                .clone()
                .unwrap_or_else(|| t.profile_placeholder.to_string());
            egui::ComboBox::from_id_salt("ssh-profile")
                .selected_text(current)
                .show_ui(ui, |ui| {
                    for prof in app.cfg.profiles.clone() {
                        if ui.selectable_label(false, &prof.name).clicked() {
                            app.form.profile_name = Some(prof.name.clone());
                            load_profile_into(app, &prof);
                        }
                    }
                });
            if let Some(name) = app.form.profile_name.clone() {
                if ui
                    .add(egui::Button::new(RichText::new(t.profile_delete).size(12.0)))
                    .clicked()
                {
                    app.cfg.delete_profile(&name);
                    app.form.profile_name = None;
                }
            }
            ui.add_space(6.0);
        }

        let form = &mut app.form;
        components::field(ui, p, t.field_host, &mut form.host, t.hint_host);
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width() * 0.3, 60.0),
                Layout::left_to_right(Align::Min),
                |ui| components::field(ui, p, t.field_port, &mut form.port, ""),
            );
            ui.allocate_ui_with_layout(
                ui.available_size(),
                Layout::left_to_right(Align::Min),
                |ui| components::field(ui, p, t.field_user, &mut form.user, t.hint_user),
            );
        });
        ui.add_space(2.0);

        let mut key = form.auth_key;
        components::segmented(ui, p, &[(false, t.auth_password), (true, t.auth_key)], &mut key);
        form.auth_key = key;

        if form.auth_key {
            components::field(ui, p, t.field_key_path, &mut form.key_path, "");
            components::password_field(ui, p, t.field_passphrase, &mut form.passphrase);
        } else {
            components::password_field(ui, p, t.field_password, &mut form.password);
        }
        ui.add_space(2.0);

        if ui.checkbox(&mut form.use_sudo, t.use_sudo).on_hover_text(t.use_sudo_hint).changed() {
            // rien de plus à faire : l'état est lu à la connexion
        }
        if form.use_sudo {
            components::password_field(ui, p, t.field_sudo_password, &mut form.sudo_password);
        }
        ui.add_space(4.0);

        // Enregistrement du profil (emprunt de form terminé avant d'appeler app).
        let ready = {
            let f = &mut app.form;
            !f.host.trim().is_empty() && !f.user.trim().is_empty()
        };
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 6.0);
            if ui
                .add_enabled(
                    ready,
                    egui::TextEdit::singleline(&mut app.form.save_name)
                        .hint_text(t.save_profile_placeholder)
                        .desired_width((ui.available_width() - 140.0).clamp(140.0, 220.0)),
                )
                .lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter))
            {
                save_profile(app);
            }
            if ui
                .add_enabled(ready, egui::Button::new(t.save_profile_btn))
                .clicked()
            {
                save_profile(app);
            }
        });
        ui.add_space(6.0);

        if app.phase == ConnPhase::Connecting && app.form.mode_ssh {
            components::loading(ui, p, t.status_connecting);
        } else if ui
            .add_enabled(ready, primary_button(p, t))
            .clicked()
        {
            connect_ssh(app);
        }
        ui.add_space(2.0);
        components::hint(ui, p, t.ssh_fingerprint_hint);
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

fn primary_button(p: &crate::theme::Palette, t: &crate::i18n::Tr) -> egui::Button<'static> {
    egui::Button::new(
        RichText::new(t.connect_btn)
            .color(egui::Color32::WHITE)
            .family(egui::FontFamily::Name("Medium".into())),
    )
    .fill(p.accent)
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