use eframe::egui::{self, RichText};

use crate::ui::theme;

/// A titled card that groups related controls, as wide as the room it's in.
pub fn card<R>(ui: &mut egui::Ui, title: &str, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
    framed(ui, title, 0.0, add_contents).0
}

/// Cards side by side, splitting the width evenly and all as tall as the
/// tallest, like a flex row. Add `count` cards with [`CardRow::card`].
///
/// egui lays out in one pass, so a card can't know how tall the next one will
/// be. The row remembers the tallest from the frame before, and when that
/// changes it asks egui to draw the frame again before showing it.
pub fn card_row(
    ui: &mut egui::Ui,
    id_salt: impl egui::AsIdSalt,
    count: usize,
    add_cards: impl FnOnce(&mut CardRow),
) {
    let id = ui.make_persistent_id(id_salt);
    let remembered = ui.data(|d| d.get_temp::<f32>(id)).unwrap_or(0.0);

    let tallest = ui
        .horizontal_top(|ui| {
            let gaps = ui.spacing().item_spacing.x * (count.saturating_sub(1)) as f32;
            let mut row = CardRow {
                width: (ui.available_width() - gaps) / count.max(1) as f32,
                min_height: remembered,
                tallest: 0.0,
                ui,
            };
            add_cards(&mut row);
            row.tallest
        })
        .inner;

    if (tallest - remembered).abs() > 0.5 {
        ui.data_mut(|d| d.insert_temp(id, tallest));
        ui.ctx().request_discard("card row height changed");
    }
}

/// The cards of a [`card_row`].
pub struct CardRow<'a> {
    ui: &'a mut egui::Ui,
    width: f32,
    min_height: f32,
    /// The tallest card's contents this frame, before stretching.
    tallest: f32,
}

impl CardRow<'_> {
    pub fn card<R>(&mut self, title: &str, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
        let (inner, height) = self
            .ui
            .vertical(|ui| {
                ui.set_width(self.width);
                framed(ui, title, self.min_height, add_contents)
            })
            .inner;
        self.tallest = self.tallest.max(height);
        inner
    }
}

/// A card at least `min_height` tall inside its padding. Also returns how
/// tall its contents are on their own, which is what a row measures.
fn framed<R>(
    ui: &mut egui::Ui,
    title: &str,
    min_height: f32,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> (R, f32) {
    theme::card_frame()
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(min_height);
            let contents = ui.vertical(|ui| {
                ui.label(
                    RichText::new(title.to_uppercase())
                        .font(theme::label_font())
                        .color(theme::TEXT_DIM)
                        .strong(),
                );
                add_contents(ui)
            });
            (contents.inner, contents.response.rect.height())
        })
        .inner
}
