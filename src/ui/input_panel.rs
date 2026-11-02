use std::sync::atomic::Ordering;

use eframe::egui::{self, Popup, RichText};

use crate::{
    audio::studio::{input::MAX_TRIM, studio::MAX_INPUTS},
    shared::{AppParams, InputDevice, InputHandle, input_devices},
    ui::{
        theme,
        widgets::{
            knob::Knob,
            list::{add_card, list_card, menu_frame, menu_option, section_label},
            value::Unit,
        },
    },
};

/// UI-side state of the input list: the devices offered in the add menu.
#[derive(Default)]
pub struct InputPanel {
    /// Looked up when the add menu opens, because asking the system for its
    /// devices every frame would be slow.
    devices: Vec<InputDevice>,
}

impl InputPanel {
    /// Lists the inputs. Clicking one turns it on or off; right-clicking picks
    /// the channel it plays into. A last card with a + adds a device.
    ///
    /// Returns the device to add, if one was picked.
    pub fn show(&mut self, ui: &mut egui::Ui, params: &AppParams) -> Option<InputDevice> {
        let mut picked = None;

        section_label(ui, "INPUTS");

        for (i, input) in params.inputs.iter().enumerate() {
            input_card(ui, i, input, params);
        }

        if params.inputs.len() < MAX_INPUTS {
            let add = add_card(ui, "add_input").on_hover_text("Add input");
            if add.clicked() {
                self.devices = input_devices();
            }

            Popup::menu(&add)
                .width(add.rect.width())
                .gap(4.0)
                .frame(menu_frame())
                .show(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    if self.devices.is_empty() {
                        ui.label(RichText::new("No input devices").color(theme::TEXT_DIM));
                    }
                    for (i, device) in self.devices.iter().enumerate() {
                        let label = device.channel_label().unwrap_or_default();
                        if menu_option(ui, &device.name, &label) {
                            picked = Some(i);
                            ui.close();
                        }
                    }
                });
        }

        // taken out of the list; it's looked up again next time the menu opens
        picked.map(|i| self.devices.swap_remove(i))
    }
}

/// One input: its name, whether it's on, and where it plays into.
fn input_card(ui: &mut egui::Ui, index: usize, input: &InputHandle, params: &AppParams) {
    let enabled = input.params.enabled.load(Ordering::Relaxed);
    let current = input.params.route.get_channel();
    let output = current
        .and_then(|id| params.channel(id))
        .map_or("no output", |c| c.name.as_str());

    let response = list_card(ui, ("input", index), enabled, |ui| {
        ui.add(egui::Label::new(RichText::new(&input.name).strong()).truncate());
        let state = if enabled { "On" } else { "Off" };
        ui.label(
            RichText::new(format!("{state} · → {output}"))
                .font(theme::value_font())
                .color(if enabled {
                    theme::TEXT
                } else {
                    theme::TEXT_DIM
                }),
        );
    })
    .on_hover_text("Click to turn on or off, right-click for trim and output");

    if response.clicked() {
        input.params.enabled.store(!enabled, Ordering::Relaxed);
    }

    // trim and route are atomics, so they can be changed right here
    response.context_menu(|ui| {
        ui.add(
            Knob::new(&input.params.trim, 1.0..=MAX_TRIM, "Trim", Unit::Decibels).logarithmic(true),
        );
        ui.separator();
        ui.label(
            RichText::new("OUTPUT")
                .font(theme::label_font())
                .color(theme::TEXT_DIM),
        );
        for channel in &params.channels {
            let id = channel.id();
            if ui
                .selectable_label(current == Some(id), &channel.name)
                .clicked()
            {
                input.params.route.set_channel(Some(id));
                ui.close();
            }
        }
        if ui
            .selectable_label(current.is_none(), "No output")
            .clicked()
        {
            input.params.route.set_channel(None);
            ui.close();
        }
    });
}
