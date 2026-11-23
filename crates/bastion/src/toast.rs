//! Notifications éphémères (toasts) : succès, erreur, information.
//! Les erreurs restent affichées jusqu'à fermeture manuelle.
//!
//! Les toasts sont ancrés au-dessus de la barre d'état et de la navigation
//! basse : ils ne recouvrent jamais un contrôle.

use crate::theme::Palette;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Success,
    Error,
    Info,
}

/// Action réversible proposée dans un toast (« Annuler »).
#[derive(Debug, Clone)]
pub enum ToastAction {
    Reban { jail: String, ip: String },
    Reunban { jail: String, ip: String },
}

pub struct Toast {
    kind: ToastKind,
    text: String,
    action: Option<ToastAction>,
    born: Instant,
    closed: bool,
}

#[derive(Default)]
pub struct Toasts {
    items: Vec<Toast>,
}

impl Toasts {
    fn push(&mut self, kind: ToastKind, text: String, action: Option<ToastAction>) {
        self.items.push(Toast {
            kind,
            text,
            action,
            born: Instant::now(),
            closed: false,
        });
        // Les toasts s'empilent : on garde au plus 4 entrées lisibles.
        while self.items.len() > 4 {
            self.items.remove(0);
        }
    }

    pub fn success(&mut self, text: impl Into<String>) {
        self.push(ToastKind::Success, text.into(), None);
    }

    pub fn info(&mut self, text: impl Into<String>) {
        self.push(ToastKind::Info, text.into(), None);
    }

    pub fn error(&mut self, text: impl Into<String>) {
        self.push(ToastKind::Error, text.into(), None);
    }

    pub fn success_undo(&mut self, text: impl Into<String>, action: ToastAction) {
        self.push(ToastKind::Success, text.into(), Some(action));
    }

    /// Affiche la pile de toasts (bas-droite, au-dessus de `bottom_inset`) et
    /// gère l'expiration. Retourne les actions « Annuler » cliquées.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        p: &Palette,
        t: &crate::i18n::Tr,
        bottom_inset: f32,
    ) -> Vec<ToastAction> {
        // Expiration : succès/info disparaissent après 5 s.
        for toast in &mut self.items {
            if toast.closed {
                continue;
            }
            if matches!(toast.kind, ToastKind::Success | ToastKind::Info)
                && toast.born.elapsed() >= Duration::from_secs(5)
            {
                toast.closed = true;
            }
        }
        self.items.retain(|t| !t.closed);

        let mut actions: Vec<ToastAction> = Vec::new();

        if !self.items.is_empty() {
            let viewport = ctx.input(|i| i.viewport_rect());
            egui::Area::new(egui::Id::unique("toasts"))
                .anchor(
                    egui::Align2::RIGHT_BOTTOM,
                    egui::vec2(-16.0, -(bottom_inset + 12.0)),
                )
                .constrain_to(viewport)
                .order(egui::Order::Foreground)
                .interactable(true)
                .show(ctx, |ui| {
                    ui.set_max_width((viewport.width() - 32.0).clamp(240.0, 400.0));
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                    for toast in self.items.iter_mut().rev() {
                        render_toast(ui, p, t, toast, &mut actions);
                    }
                });
            self.items.retain(|t| !t.closed);
        }
        actions
    }
}

fn render_toast(
    ui: &mut egui::Ui,
    p: &Palette,
    t: &crate::i18n::Tr,
    toast: &mut Toast,
    actions: &mut Vec<ToastAction>,
) {
    let (tone, icon) = match toast.kind {
        ToastKind::Success => (p.success, crate::icons::CHECK_CIRCLE),
        ToastKind::Error => (p.danger, crate::icons::ERROR),
        ToastKind::Info => (p.accent, crate::icons::INFO),
    };

    let frame = egui::Frame::NONE
        .fill(p.card)
        .stroke(egui::Stroke::new(1.0, p.border))
        .corner_radius(egui::CornerRadius::same(11))
        .inner_margin(egui::Margin::same(12))
        .shadow(egui::Shadow {
            offset: [0, 4],
            blur: 18,
            spread: 0,
            color: egui::Color32::from_black_alpha(if p.dark { 110 } else { 45 }),
        });
    frame.show(ui, |ui| {
        ui.set_width((ui.ctx().input(|i| i.viewport_rect()).width() - 56.0).clamp(200.0, 360.0));
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 4.0);
                ui.horizontal(|ui| {
                    ui.label(crate::icons::icon_sz(icon, 15.0, tone));
                    ui.label(
                        egui::RichText::new(&toast.text)
                            .size(13.0)
                            .color(p.text)
                            .family(egui::FontFamily::Name("Medium".into())),
                    );
                });
                if let Some(action) = toast.action.clone() {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(format!("↺  {}", t.undo_btn))
                                    .size(12.0)
                                    .color(p.accent)
                                    .family(egui::FontFamily::Name("Medium".into())),
                            )
                            .frame(false),
                        )
                        .on_hover_text(t.undo_tooltip)
                        .clicked()
                    {
                        actions.push(action);
                        toast.closed = true;
                    }
                }
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                if ui
                    .add(
                        egui::Button::new(
                            crate::icons::icon_sz(
                                crate::icons::CLOSE,
                                13.0,
                                p.text_secondary,
                            )
                            .family(egui::FontFamily::Name("Icon".into())),
                        )
                        .frame(false),
                    )
                    .clicked()
                {
                    toast.closed = true;
                }
            });
        });
    });
}
