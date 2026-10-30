//! Modales : confirmations destructrices, rapport, journaux système,
//! validation de la clé de serveur SSH (TOFU). L'ajout de règle est
//! inline dans l'écran Pare-feu (pas de popin).

use crate::app::{ConfirmKind, ConnPhase, GuiApp, HostKeyCtx, Modal, Pending};
use crate::components;
use crate::theme::Palette;
use bastion_core::op::{Op, ServiceAction};
use egui::{Align, Color32, Layout, RichText};

/// Rend une modale centrée (fond assombri, clic extérieur = annulation).
/// Retourne vrai si l'utilisateur a cliqué dans le fond.
fn show_modal(
    ctx: &egui::Context,
    p: &Palette,
    id: &str,
    title: &str,
    min_width: f32,
    body: impl FnOnce(&mut egui::Ui),
) -> bool {
    // La largeur demandée est bornée à l'écran : les monospace longs ne
    // doivent jamais pousser la modale hors de la fenêtre.
    let max = (ctx.input(|i| i.viewport_rect().width()) - 48.0).max(280.0);
    let width = min_width.min(max);
    let resp = egui::Modal::new(egui::Id::unique(id))
        .frame(components::modal_frame(p))
        .backdrop_color(Color32::from_black_alpha(170))
        .show(ctx, |ui| {
            ui.set_min_width(width);
            ui.set_max_width(width);
            components::modal_title(ui, p, title);
            body(ui);
        });
    resp.backdrop_response.clicked()
}

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
                    if ui.add(danger_btn(p, &confirm_label)).clicked() {
                        apply_confirm(app, &data.kind);
                        close = true;
                    }
                    if ui.button(app.tr.cancel_btn).clicked() {
                        close = true;
                    }
                });
            });
            if !close && !backdrop {
                app.modal = Some(Modal::Confirm(data));
            }
        }

        Modal::ResetFw { mut typed } => {
            let mut close = false;
            let t = app.tr;
            let backdrop = show_modal(ctx, p, "reset-fw", t.reset_title, 460.0, |ui| {
                ui.label(t.reset_body);
                ui.add_space(10.0);
                ui.label(
                    RichText::new(t.reset_type_prompt)
                        .size(12.5)
                        .color(p.text_secondary),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut typed)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(260.0),
                );
                ui.add_space(12.0);
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    let enabled = typed.trim().to_uppercase() == t.reset_word;
                    if ui.add_enabled(enabled, danger_btn(p, t.reset_btn.trim_end_matches('…'))).clicked() {
                        app.send_op(Pending::Reset, Op::UfwReset);
                        close = true;
                    }
                    if ui.button(t.cancel_btn).clicked() {
                        close = true;
                    }
                });
            });
            if !close && !backdrop {
                app.modal = Some(Modal::ResetFw { typed });
            }
        }

        Modal::Report(text) => {
            let mut close = false;
            let t = app.tr;
            let backdrop = show_modal(
                ctx,
                p,
                "report",
                &format!("{} — ufw show listening", t.report_btn),
                720.0,
                |ui| {
                    egui::ScrollArea::both()
                        .max_height(420.0)
                        .min_scrolled_height(420.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| ui.monospace(&text));
                    ui.add_space(8.0);
                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                        if ui.button(t.close_btn).clicked() {
                            close = true;
                        }
                    });
                },
            );
            if !close && !backdrop {
                app.modal = Some(Modal::Report(text));
            }
        }

        Modal::Logs { title, text } => {
            let mut close = false;
            let t = app.tr;
            let backdrop = show_modal(ctx, p, "sys-logs", &title, 760.0, |ui| {
                egui::ScrollArea::both()
                    .max_height(440.0)
                    .min_scrolled_height(440.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| ui.monospace(&text));
                ui.add_space(8.0);
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    if ui
                        .add(egui::Button::new(components::btn_icon_text(crate::icons::COPY, t.copy_btn)))
                        .clicked()
                    {
                        ui.ctx().copy_text(text.clone());
                        app.toasts.info(t.journal_copied_toast.to_string());
                    }
                    if ui.button(t.close_btn).clicked() {
                        close = true;
                    }
                });
            });
            if !close && !backdrop {
                app.modal = Some(Modal::Logs { title, text });
            }
        }

        Modal::HostKey(HostKeyCtx { host, fingerprint, old, params }) => {
            let t = app.tr;
            if fingerprint.is_empty() {
                // En attente du résultat de connexion.
                app.modal = Some(Modal::HostKey(HostKeyCtx { host, fingerprint, old, params }));
                return;
            }
            let mut close = false;
            let title = if old.is_some() { t.hk_changed_title } else { t.hk_unknown_title };
            let backdrop = show_modal(ctx, p, "hostkey", title, 540.0, |ui| {
                if let Some(old_fp) = &old {
                    ui.colored_label(p.danger, t.hk_mitm_warning);
                    ui.add_space(8.0);
                    ui.label(RichText::new(t.hk_stored_fp).size(11.5).color(p.text_secondary));
                    ui.monospace(old_fp);
                    ui.label(RichText::new(t.hk_new_fp).size(11.5).color(p.text_secondary));
                    ui.monospace(&fingerprint);
                } else {
                    ui.label(t.first_time_key(&host));
                    ui.add_space(6.0);
                    ui.label(RichText::new(t.hk_fp_label).size(11.5).color(p.text_secondary));
                    ui.monospace(&fingerprint);
                    ui.add_space(6.0);
                    components::hint(ui, p, t.hk_verify_hint);
                }
                ui.add_space(12.0);
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    if components::primary_button(ui, p, t.hk_trust_btn).clicked() {
                        app.cfg.remember_host(&host, params.port, &fingerprint);
                        let mut p2 = params.clone();
                        p2.expected_fingerprint = Some(fingerprint.clone());
                        app.phase = ConnPhase::Connecting;
                        app.worker.send(crate::worker::Job::ConnectSsh { params: p2 });
                        close = true;
                    }
                    if ui.button(t.cancel_btn).clicked() {
                        close = true;
                    }
                });
            });
            if !close && !backdrop {
                app.modal = Some(Modal::HostKey(HostKeyCtx { host, fingerprint, old, params }));
            }
        }
    }
}

fn apply_confirm(app: &mut GuiApp, kind: &ConfirmKind) {
    match kind {
        ConfirmKind::DisableFw => app.send_op(Pending::Disable, Op::UfwDisable),
        ConfirmKind::DeleteRule { index } => {
            app.send_op(Pending::DeleteRule, Op::UfwDeleteRule { index: *index });
        }
        ConfirmKind::StopF2b => app.send_op(
            Pending::Service(ServiceAction::Stop),
            Op::F2bServiceAction { action: ServiceAction::Stop },
        ),
        ConfirmKind::RestartF2b => app.send_op(
            Pending::Service(ServiceAction::Restart),
            Op::F2bServiceAction { action: ServiceAction::Restart },
        ),
        ConfirmKind::StopUfw => app.send_op(
            Pending::UfwService(ServiceAction::Stop),
            Op::UfwServiceAction { action: ServiceAction::Stop },
        ),
        ConfirmKind::RestartUfw => app.send_op(
            Pending::UfwService(ServiceAction::Restart),
            Op::UfwServiceAction { action: ServiceAction::Restart },
        ),
        ConfirmKind::StopClamav { unit } => app.send_op(
            Pending::ClamService {
                unit: unit.clone(),
                action: ServiceAction::Stop,
            },
            Op::ClamServiceAction {
                unit: unit.clone(),
                action: ServiceAction::Stop,
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
