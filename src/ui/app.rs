use std::collections::BTreeSet;

use eframe::egui::{self, Align, Layout, RichText};

use crate::{
    events::NoteEvent,
    shared::AppParams,
    ui::{
        keyboard::{self, key_to_note_event},
        mixer_panel, synth_panel, theme,
        widgets::piano::piano,
    },
};

pub struct App {
    note_producer: rtrb::Producer<NoteEvent>,
    params: AppParams,
    held_keys: BTreeSet<u8>,
    /// Note currently played by clicking the on-screen piano.
    mouse_note: Option<u8>,
}

impl App {
    pub fn new(note_producer: rtrb::Producer<NoteEvent>, params: AppParams) -> Self {
        Self {
            note_producer,
            held_keys: BTreeSet::new(),
            mouse_note: None,
            params,
        }
    }

    fn handle_keys(&mut self, ui: &egui::Ui) {
        ui.input(|i| {
            for ev in &i.events {
                if let Some(note_event) = key_to_note_event(ev) {
                    match note_event {
                        NoteEvent::On(n) => {
                            self.held_keys.insert(n);
                        }
                        NoteEvent::Off(n) => {
                            self.held_keys.remove(&n);
                        }
                    }

                    let _ = self.note_producer.push(note_event);
                }
            }
        });
    }

    fn set_mouse_note(&mut self, note: Option<u8>) {
        if note == self.mouse_note {
            return;
        }
        if let Some(n) = self.mouse_note {
            let _ = self.note_producer.push(NoteEvent::Off(n));
        }
        if let Some(n) = note {
            let _ = self.note_producer.push(NoteEvent::On(n));
        }
        self.mouse_note = note;
    }

    fn header(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let (dot, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
            ui.painter().circle_filled(dot.center(), 5.0, theme::ACCENT);
            ui.label(RichText::new("Vincent Studio").heading().strong());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let held = self.held_keys.len() + self.mouse_note.is_some() as usize;
                ui.label(
                    RichText::new(format!("{held} held"))
                        .font(theme::value_font())
                        .color(if held > 0 {
                            theme::ACCENT
                        } else {
                            theme::TEXT_DIM
                        }),
                );
            });
        });
    }

    fn keyboard(&mut self, ui: &mut egui::Ui) {
        let mut held = self.held_keys.clone();
        held.extend(self.mouse_note);

        let note = piano(ui, keyboard::NOTE_RANGE, &held, |n| {
            keyboard::note_to_key(n).map(|k| k.name().to_owned())
        });
        self.set_mouse_note(note);
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.handle_keys(ui);

        egui::Panel::top("header")
            .frame(theme::panel_frame())
            .resizable(false)
            .show(ui, |ui| self.header(ui));

        egui::Panel::bottom("keyboard")
            .frame(theme::panel_frame())
            .resizable(false)
            .show(ui, |ui| self.keyboard(ui));

        egui::CentralPanel::default()
            .frame(egui::Frame::new().inner_margin(16))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(theme::GAP, theme::GAP);
                let layout = Layout::left_to_right(Align::Min).with_main_wrap(true);
                ui.with_layout(layout, |ui| {
                    synth_panel::show(ui, &self.params.synth);
                    mixer_panel::show(ui, &self.params.mixer);
                });
            });
    }
}
