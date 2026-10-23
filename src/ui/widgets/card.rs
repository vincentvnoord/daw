use eframe::egui::{self, RichText};

use crate::ui::theme;

/// A titled card that groups related controls.
pub fn card<R>(ui: &mut egui::Ui, title: &str, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
    theme::card_frame()
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(title.to_uppercase())
                        .font(theme::label_font())
                        .color(theme::TEXT_DIM)
                        .strong(),
                );
                ui.add_space(6.0);
                add_contents(ui)
            })
            .inner
        })
        .inner
}
