//! Modales : confirmations destructrices, ajout de règle, rapport,
//! journaux système, validation de clé de serveur SSH (TOFU).

use crate::app::{AddRuleForm, ConfirmKind, GuiApp, HostKeyCtx, Modal, Pending};
use crate::components;
use crate::theme::Palette;
use egui::{Align, Color32, Layout, RichText};
use gufw_core::model::*;
use gufw_core::op::{Op, ServiceAction};

/// Rend une modale (fond assombri, clic extérieur = annuler).
/// Retourne vrai si l'utilisateur a cliqué dans le fond (annuler).
#[allow(deprecated)] // Modal::new exige un Id global
fn show_modal(
    ctx: &egui::Context,
    p: &Palette,
    id: &str,
    title: &str,
    min_width: f32,
    body: impl FnOnce(&mut egui::Ui),
) -> bool {
    let resp = egui::Modal::new(egui::Id::new(id))
        .frame(components::modal_frame(p))
        .backdrop_color(Color32::from_black_alpha(170))
        .show(ctx, |ui| {
            ui.set_min_width(min_width);
            components::modal_title(ui, p, title);
            body(ui);
        });
    resp.backdrop_response.clicked()
}

/// Rend la modale courante ; la referme (`app.modal` reste à `None`) ou la
/// réinsère selon les interactions.
pub fn render(app: &mut GuiApp, ctx: &egui::Context, p: &Palette, modal: Modal) {
    match modal {
        Modal::Confirm(data) => {
            let mut close = false;
            let title = data.title.clone();
            let body = data.body.clone();
            let confirm_label = data.confirm_label.clone();
            let backdrop = show_modal(ctx, p, "confirm", &title, 440.0, |ui| {
                ui.label(body);
                ui.add_space(16.0);
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(confirm_label)
                                    .color(Color32::WHITE)
                                    .family(egui::FontFamily::Name("Medium".into())),
                            )
                            .fill(p.danger),
                        )
                        .clicked()
                    {
                        apply_confirm(app, &data.kind);
                        close = true;
                    }
                    if ui.button("Annuler").clicked() {
                        close = true;
                    }
                });
            });
            close = close || backdrop;
            if !close {
                app.modal = Some(Modal::Confirm(data));
            }
        }

        Modal::ResetFw { mut typed } => {
            let mut close = false;
            let backdrop = show_modal(
                ctx,
                p,
                "reset-fw",
                "Réinitialiser le pare-feu ?",
                460.0,
                |ui| {
                    ui.label(
                        "Toutes les règles UFW seront supprimées et les politiques par défaut rétablies. Cette action est irréversible.",
                    );
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new("Tapez RÉINITIALISER pour confirmer :")
                            .size(12.5)
                            .color(p.text_secondary),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut typed)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(240.0),
                    );
                    ui.add_space(12.0);
                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                        let enabled = typed.trim() == "RÉINITIALISER";
                        if ui
                            .add_enabled(enabled, danger_btn(p, "Réinitialiser"))
                            .clicked()
                        {
                            app.send_op(Pending::Reset, Op::UfwReset);
                            close = true;
                        }
                        if ui.button("Annuler").clicked() {
                            close = true;
                        }
                    });
                },
            );
            close = close || backdrop;
            if !close {
                app.modal = Some(Modal::ResetFw { typed });
            }
        }

        Modal::AddRule => {
            let mut close = false;
            let backdrop = show_modal(ctx, p, "add-rule", "Ajouter une règle", 580.0, |ui| {
                let f = &mut app.add_form;

                // Action et sens (segmentés).
                ui.horizontal(|ui| {
                    let mut action = f.action;
                    components::segmented(
                        ui,
                        p,
                        &[
                            (RuleAction::Allow, "Autoriser"),
                            (RuleAction::Deny, "Bloquer"),
                            (RuleAction::Reject, "Rejeter"),
                            (RuleAction::Limit, "Limiter"),
                        ],
                        &mut action,
                    );
                    f.action = action;
                });
                ui.horizontal(|ui| {
                    let mut dir = f.dir;
                    components::segmented(
                        ui,
                        p,
                        &[(NewRuleDir::In, "Entrée"), (NewRuleDir::Out, "Sortie")],
                        &mut dir,
                    );
                    f.dir = dir;
                });
                ui.add_space(6.0);

                if app.firewall.profiles.is_empty() {
                    rule_fields(ui, p, f);
                } else {
                    ui.checkbox(&mut f.use_profile, "Utiliser un profil d'application");
                    if f.use_profile {
                        let sel = f
                            .profile
                            .clone()
                            .unwrap_or_else(|| app.firewall.profiles.first().cloned().unwrap_or_default());
                        let mut chosen = sel.clone();
                        egui::ComboBox::from_id_salt("rule-profile")
                            .selected_text(&sel)
                            .width(240.0)
                            .show_ui(ui, |ui| {
                                for prof in &app.firewall.profiles {
                                    ui.selectable_value(&mut chosen, prof.clone(), prof);
                                }
                            });
                        f.profile = Some(chosen);
                    } else {
                        rule_fields(ui, p, f);
                    }
                }

                // Options avancées (révélées, jamais imposées).
                ui.add_space(4.0);
                if ui
                    .add(egui::Button::new(
                        RichText::new(format!("{}  Options avancées", crate::icons::EXPAND_MORE))
                            .size(12.5)
                            .family(egui::FontFamily::Name("Icon".into())),
                    ))
                    .clicked()
                {
                    f.advanced = !f.advanced;
                }
                if f.advanced {
                    components::field(ui, p, "Adresse source (CIDR, vide = partout)", &mut f.from_addr, "192.168.1.0/24");
                    components::field(ui, p, "Adresse destination (vide = partout)", &mut f.to_addr, "10.0.0.5");
                    components::field(ui, p, "Position d'insertion (vide = à la fin)", &mut f.insert_at, "1, 2, 3…");
                    components::field(ui, p, "Commentaire", &mut f.comment, "Affiché dans la table des règles");
                }

                // Aperçu exact (double lecture : l'expert lit la commande).
                ui.add_space(6.0);
                let rule_opt = f.to_new_rule().ok();
                let preview = f.preview();
                ui.label(RichText::new("Aperçu :").size(11.5).color(p.text_secondary));
                ui.monospace(&preview);

                ui.add_space(10.0);
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    if ui
                        .add_enabled(rule_opt.is_some(), primary_btn(p, "Ajouter"))
                        .clicked()
                    {
                        if let Some(rule) = rule_opt {
                            app.send_op(Pending::AddRule, Op::UfwAddRule { rule });
                        }
                        close = true;
                    }
                    if ui.button("Annuler").clicked() {
                        close = true;
                    }
                });
            });
            close = close || backdrop;
            if close {
                app.add_form = AddRuleForm::default();
            } else {
                app.modal = Some(Modal::AddRule);
            }
        }

        Modal::Report(text) => {
            let mut close = false;
            let backdrop = show_modal(ctx, p, "report", "Rapport UFW", 720.0, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(420.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.monospace(&text);
                    });
                ui.add_space(8.0);
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    if ui.button("Fermer").clicked() {
                        close = true;
                    }
                });
            });
            close = close || backdrop;
            if !close {
                app.modal = Some(Modal::Report(text));
            }
        }

        Modal::Logs { title, text } => {
            let mut close = false;
            let backdrop = show_modal(ctx, p, "sys-logs", &title, 760.0, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(440.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.monospace(&text);
                    });
                ui.add_space(8.0);
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    if ui
                        .add(egui::Button::new(components::btn_icon_text(
                            crate::icons::COPY,
                            "Copier",
                        )))
                        .clicked()
                    {
                        ui.ctx().copy_text(text.clone());
                        app.toasts.info("Journal copié");
                    }
                    if ui.button("Fermer").clicked() {
                        close = true;
                    }
                });
            });
            close = close || backdrop;
            if !close {
                app.modal = Some(Modal::Logs { title, text });
            }
        }

        Modal::HostKey(ctx_data) => {
            let HostKeyCtx {
                host,
                fingerprint,
                old,
                params,
            } = ctx_data;

            if fingerprint.is_empty() {
                // En attente du résultat de connexion : la modale sera
                // renseignée par l'événement (HostKeyUnknown/Changed).
                app.modal = Some(Modal::HostKey(HostKeyCtx {
                    host,
                    fingerprint,
                    old,
                    params,
                }));
                return;
            }

            let mut close = false;
            let title = if old.is_some() {
                "Clé du serveur modifiée"
            } else {
                "Clé du serveur inconnue"
            };
            let backdrop = show_modal(ctx, p, "hostkey", title, 540.0, |ui| {
                if let Some(old) = &old {
                    ui.colored_label(
                        p.danger,
                        "La clé présentée par ce serveur diffère de celle mémorisée. Une attaque de type « man-in-the-middle » est possible.",
                    );
                    ui.add_space(8.0);
                    ui.label(RichText::new("Empreinte mémorisée :").size(11.5).color(p.text_secondary));
                    ui.monospace(old);
                    ui.label(RichText::new("Nouvelle empreinte :").size(11.5).color(p.text_secondary));
                    ui.monospace(&fingerprint);
                } else {
                    ui.label(format!(
                        "La connexion à « {host} » présente cette clé pour la première fois."
                    ));
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("Empreinte SHA256 de la clé du serveur :")
                            .size(11.5)
                            .color(p.text_secondary),
                    );
                    ui.monospace(&fingerprint);
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(
                            "Vérifiez-la auprès de votre hébergeur ou sur la console du serveur (ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub) avant de continuer.",
                        )
                        .size(12.5),
                    );
                }
                ui.add_space(12.0);
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    if components::primary_button(ui, p, "Faire confiance et se connecter").clicked() {
                        app.cfg.remember_host(&host, params.port, &fingerprint);
                        let mut params = params.clone();
                        params.expected_fingerprint = Some(fingerprint.clone());
                        app.phase = crate::app::ConnPhase::Connecting;
                        app.worker.send(crate::worker::Job::ConnectSsh { params });
                        close = true;
                    }
                    if ui.button("Annuler").clicked() {
                        close = true;
                    }
                });
            });
            close = close || backdrop;
            if !close {
                app.modal = Some(Modal::HostKey(HostKeyCtx {
                    host,
                    fingerprint,
                    old,
                    params,
                }));
            }
        }
    }
}

/// Champs port + protocole du formulaire de règle.
fn rule_fields(ui: &mut egui::Ui, p: &Palette, f: &mut AddRuleForm) {
    components::field(ui, p, "Port / service", &mut f.port, "22, 8080, 8000:8010, ssh…");
    let mut proto = f.proto;
    components::segmented(
        ui,
        p,
        &[
            (Proto::Tcp, "TCP"),
            (Proto::Udp, "UDP"),
            (Proto::Any, "Les deux"),
        ],
        &mut proto,
    );
    f.proto = proto;
}

fn apply_confirm(app: &mut GuiApp, kind: &ConfirmKind) {
    match kind {
        ConfirmKind::DisableFw => app.send_op(Pending::Disable, Op::UfwDisable),
        ConfirmKind::DeleteRule { index } => {
            let index = *index;
            app.send_op(Pending::DeleteRule, Op::UfwDeleteRule { index })
        }
        ConfirmKind::StopF2b => app.send_op(
            Pending::Service(ServiceAction::Stop),
            Op::F2bServiceAction {
                action: ServiceAction::Stop,
            },
        ),
        ConfirmKind::RestartF2b => app.send_op(
            Pending::Service(ServiceAction::Restart),
            Op::F2bServiceAction {
                action: ServiceAction::Restart,
            },
        ),
    }
}

fn danger_btn(p: &Palette, label: &str) -> egui::Button<'static> {
    egui::Button::new(
        RichText::new(label)
            .color(Color32::WHITE)
            .family(egui::FontFamily::Name("Medium".into())),
    )
    .fill(p.danger)
}

fn primary_btn(p: &Palette, label: &str) -> egui::Button<'static> {
    egui::Button::new(
        RichText::new(label)
            .color(Color32::WHITE)
            .family(egui::FontFamily::Name("Medium".into())),
    )
    .fill(p.accent)
}
