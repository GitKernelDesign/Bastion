//! Écran Connexion : cible locale ou SSH, profils enregistrés, état de session.

use crate::app::{ConnPhase, GuiApp};
use crate::components;
use crate::config::ProfileAuth;
use crate::theme::Palette;
use crate::worker::Job;
use egui::{Align, Color32, Layout, RichText};
use gufw_core::ssh::{SshAuth, SshParams};
use std::path::PathBuf;

pub fn render(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    components::screen_header(
        ui,
        p,
        "Connexion",
        "Choisissez la machine à administrer : cet ordinateur ou un hôte distant.",
    );
    ui.add_space(6.0);

    if app.phase == ConnPhase::Connected {
        session_summary(app, ui, p);
        return;
    }

    ui.columns(2, |cols| {
        local_card(app, &mut cols[0], p);
        ssh_card(app, &mut cols[1], p);
    });

    ui.add_space(10.0);
    components::hint(
        ui,
        p,
        "Les mots de passe ne sont jamais écrits sur le disque. En local, une seule authentification "
            .to_string()
            .as_str(),
    );
    ui.label(
        RichText::new(
            "root est demandée par session (polkit). En SSH, l'empreinte de la clé du serveur est vérifiée à la première connexion.",
        )
        .size(12.5)
        .color(p.text_secondary),
    );
}

fn session_summary(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    components::card_body(ui, p, |ui| {
        ui.horizontal(|ui| {
            components::status_line(ui, p, p.success, "Session active", None);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if components::danger_button(ui, p, "Se déconnecter").clicked() {
                    app.expect_disconnect = true;
                    app.worker.send(Job::Disconnect);
                }
            });
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            components::hint(ui, p, "Cible :");
            ui.label(
                RichText::new(&app.conn_desc)
                    .monospace()
                    .family(egui::FontFamily::Name("MonoMedium".into())),
            );
        });
        ui.horizontal(|ui| {
            components::hint(ui, p, "Outils :");
            ui.label(probe_ufw(app));
            ui.label("·");
            ui.label(probe_f2b(app));
        });
        ui.add_space(4.0);
        components::hint(
            ui,
            p,
            "Les écrans Pare-feu et Fail2Ban reflètent cette cible et s'actualisent automatiquement.",
        );
    });
}

fn probe_ufw(app: &GuiApp) -> String {
    app.probe
        .0
        .clone()
        .unwrap_or_else(|| "UFW (version lue à la connexion)".into())
}

fn probe_f2b(app: &GuiApp) -> String {
    app.probe
        .1
        .clone()
        .unwrap_or_else(|| "Fail2Ban (version lue à la connexion)".into())
}

fn local_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    components::card_body(ui, p, |ui| {
        ui.horizontal(|ui| {
            ui.label(crate::icons::icon_sz(crate::icons::COMPUTER, 20.0, p.accent));
            ui.label(
                RichText::new("Cette machine")
                    .size(15.0)
                    .family(egui::FontFamily::Name("SemiBold".into())),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                components::badge(ui, p, "LOCAL", p.text_secondary, p.card_hover);
            });
        });
        ui.add_space(8.0);
        components::hint(ui, p, &format!("{}\n{}", probe_ufw(app), probe_f2b(app)));
        ui.add_space(10.0);
        if app.phase == ConnPhase::Connecting && !app.form.mode_ssh {
            components::loading(ui, p, "Demande de droits administrateur…");
        } else if components::primary_button(ui, p, "Se connecter").clicked() {
            app.form.mode_ssh = false;
            app.phase = ConnPhase::Connecting;
            let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("gufw"));
            app.worker.send(Job::ConnectLocal { app_exe: exe });
        }
        ui.add_space(2.0);
        components::hint(
            ui,
            p,
            "Une fenêtre de mot de passe apparaîtra une seule fois pour la session.",
        );
    });
}

fn ssh_card(app: &mut GuiApp, ui: &mut egui::Ui, p: &Palette) {
    components::card_body(ui, p, |ui| {
        ui.horizontal(|ui| {
            ui.label(crate::icons::icon_sz(crate::icons::TERMINAL_SSH, 20.0, p.accent));
            ui.label(
                RichText::new("À distance (SSH)")
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
                .unwrap_or_else(|| "Choisir un profil…".into());
            egui::ComboBox::from_id_salt("ssh-profile")
                .selected_text(current)
                .show_ui(ui, |ui| {
                    for prof in app.cfg.profiles.clone() {
                        if ui
                            .selectable_label(false, &prof.name)
                            .clicked()
                        {
                            app.form.profile_name = Some(prof.name.clone());
                            load_profile_into(app, &prof);
                        }
                    }
                });
            if app.form.profile_name.is_some() {
                ui.horizontal(|ui| {
                    let name = app.form.profile_name.clone().unwrap();
                    if ui
                        .add(egui::Button::new(RichText::new("Supprimer ce profil").size(12.0)))
                        .clicked()
                    {
                        app.cfg.delete_profile(&name);
                        app.form.profile_name = None;
                    }
                });
            }
            ui.add_space(6.0);
        }

        let form = &mut app.form;
        components::field(ui, p, "Hôte", &mut form.host, "Adresse IP ou nom DNS");
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width() * 0.3, 60.0),
                Layout::left_to_right(Align::Min),
                |ui| components::field(ui, p, "Port", &mut form.port, ""),
            );
            ui.allocate_ui_with_layout(
                ui.available_size(),
                Layout::left_to_right(Align::Min),
                |ui| components::field(ui, p, "Utilisateur", &mut form.user, "root, admin…"),
            );
        });
        ui.add_space(2.0);
        let mut key = form.auth_key;
        components::segmented(ui, p, &[(false, "Mot de passe"), (true, "Clé privée")], &mut key);
        form.auth_key = key;
        if form.auth_key {
            components::field(ui, p, "Fichier de clé", &mut form.key_path, "~/.ssh/id_ed25519");
            components::password_field(ui, p, "Phrase secrète (optionnelle)", &mut form.passphrase);
        } else {
            components::password_field(ui, p, "Mot de passe", &mut form.password);
        }
        ui.add_space(2.0);
        ui.checkbox(&mut form.use_sudo, "Exécuter via sudo")
            .on_hover_text("Préfixe les commandes par sudo sur l'hôte distant (si la connexion n'est pas root)");
        if form.use_sudo {
            components::password_field(ui, p, "Mot de passe sudo (optionnel)", &mut form.sudo_password);
        }
        ui.add_space(4.0);

        // Enregistrement de profil.
        ui.horizontal(|ui| {
            let ready = !form.host.trim().is_empty() && !form.user.trim().is_empty();
            ui.add_enabled(
                ready,
                egui::TextEdit::singleline(&mut form.save_name)
                    .hint_text("Enregistrer sous… (nom du profil)")
                    .desired_width(220.0),
            );
            if ui.add_enabled(ready, egui::Button::new("Enregistrer")).clicked() {
                let name = form.save_name.trim().to_string();
                if !name.is_empty() {
                    let host = form.host.trim().to_string();
                    let port = form.port.trim().parse::<u16>().unwrap_or(22);
                    let user = form.user.trim().to_string();
                    let auth = if form.auth_key {
                        ProfileAuth::Key {
                            path: form.key_path.trim().to_string(),
                        }
                    } else {
                        ProfileAuth::Password
                    };
                    app.cfg.upsert_profile(crate::config::SshProfileDef {
                        name,
                        host,
                        port,
                        user,
                        auth,
                        use_sudo: form.use_sudo,
                    });
                    app.toasts.success("Profil enregistré");
                }
            }
        });

        ui.add_space(6.0);
        let ready = !app.form.host.trim().is_empty() && !app.form.user.trim().is_empty();
        if app.phase == ConnPhase::Connecting && app.form.mode_ssh {
            components::loading(ui, p, "Connexion SSH en cours…");
        } else if ui
            .add_enabled(ready, primary_btn(p))
            .clicked()
        {
            connect_ssh(app);
        }
        ui.add_space(2.0);
        components::hint(
            ui,
            p,
            "L'empreinte de la clé du serveur sera vérifiée à la première connexion.",
        );
    });
}

fn primary_btn(p: &Palette) -> egui::Button<'static> {
    egui::Button::new(
        RichText::new("Se connecter")
            .color(Color32::WHITE)
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
            passphrase: if form.passphrase.is_empty() {
                None
            } else {
                Some(form.passphrase.clone())
            },
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
        sudo_password: if form.use_sudo && !form.sudo_password.trim().is_empty() {
            Some(form.sudo_password.trim().to_string())
        } else {
            None
        },
    };

    // Contexte de secours pour la confirmation de clé (modale).
    let ctx = crate::app::HostKeyCtx {
        host: params.host.clone(),
        fingerprint: String::new(),
        old: None,
        params: params.clone(),
    };
    app.modal = Some(crate::app::Modal::HostKey(ctx));
    app.worker.send(Job::ConnectSsh { params });
}

fn load_profile_into(app: &mut GuiApp, prof: &crate::config::SshProfileDef) {
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
