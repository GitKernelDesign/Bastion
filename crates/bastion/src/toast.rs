//! Notifications éphémères (toasts) : succès, erreur, information.
//! Les erreurs restent affichées jusqu'à fermeture manuelle.

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
        // Les toasts s'empilent : on garde au plus 5 entrées lisibles.
        while self.items.len() > 5 {
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

    /// Affiche la pile de toasts (bas-droite) et gère l'expiration.
    /// Retourne les actions « Annuler » cliquées.
    pub fn show(&mut self, ctx: &egui::Context, p: &Palette, t: &crate::i18n::Tr) -> Vec<ToastAction> {
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
            egui::Area::new(egui::Id::unique("toasts"))
                .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -16.0))
                .order(egui::Order::Foreground)
                .interactable(true)
                .show(ctx, |ui| {
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
    let (border, icon) = match toast.kind {
        ToastKind::Success => (p.success, crate::icons::CHECK_CIRCLE),
        ToastKind::Error => (p.danger, crate::icons::ERROR),
        ToastKind::Info => (p.accent, crate::icons::INFO),
    };

    let frame = egui::Frame::new()
        .fill(p.card)
        .stroke(egui::Stroke::new(1.0, border))
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::same(12));
    frame.show(ui, |ui| {
        // Largeur bornée à la fenêtre : un toast ne déborde jamais de
        // l'écran, même en fenêtre étroite (mobile).
        let max_w = (ui.ctx().input(|i| i.viewport_rect().width()) - 32.0).clamp(220.0, 380.0);
        ui.set_max_width(max_w);
        ui.horizontal(|ui| {
            ui.label(crate::icons::icon_sz(icon, 16.0, border));
            ui.label(
                egui::RichText::new(&toast.text)
                    .size(13.0)
                    .color(p.text)
                    .family(egui::FontFamily::Name("Medium".into())),
            );
        });

        // Boutons : action d'annulation + fermeture manuelle.
        if toast.action.is_some() || toast.kind == ToastKind::Error {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if let Some(action) = toast.action.clone() {
                    if ui
                        .add(egui::Button::new(
                            egui::RichText::new(t.undo_btn)
                                .color(p.accent)
                                .family(egui::FontFamily::Name("Medium".into())),
                        ))
                        .on_hover_text(t.undo_tooltip)
                        .clicked()
                    {
                        actions.push(action);
                        toast.closed = true;
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(egui::Button::new(crate::icons::icon(crate::icons::CLOSE).size(14.0)))
                        .clicked()
                    {
                        toast.closed = true;
                    }
                });
            });
        }
    });
}
