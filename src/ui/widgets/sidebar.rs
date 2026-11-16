use eframe::egui;

/// A fixed-width column on the left that scrolls on its own, and a main area
/// beside it that takes the rest of the room. Returns what each side returned.
///
/// Both sides usually need the same state, so it's handed to each in turn:
/// two closures that captured it would both borrow it at once.
pub fn sidebar<T, S, M>(
    ui: &mut egui::Ui,
    width: f32,
    state: &mut T,
    side: impl FnOnce(&mut T, &mut egui::Ui) -> S,
    main: impl FnOnce(&mut T, &mut egui::Ui) -> M,
) -> (S, M) {
    ui.horizontal_top(|ui| {
        let side = ui
            .vertical(|ui| {
                ui.set_width(width);
                egui::ScrollArea::vertical()
                    .id_salt("sidebar")
                    .auto_shrink([false, true])
                    .show(ui, |ui| side(state, ui))
                    .inner
            })
            .inner;
        let main = ui.vertical(|ui| main(state, ui)).inner;
        (side, main)
    })
    .inner
}
