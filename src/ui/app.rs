use std::collections::BTreeSet;

use eframe::egui::{self, Align, Layout, RichText};

use crate::{
    events::NoteEvent,
    shared::AppParams,
    ui::{
        editor_panel,
        instrument_panel::{self, InstrumentAction},
        keyboard::{self, key_to_note_event},
        mixer_panel::MixerPanel,
        theme,
        widgets::piano::piano,
    },
};

pub struct App {
    params: AppParams,
    /// Index of the instrument the keyboard plays.
    active: usize,
    held_keys: BTreeSet<u8>,
    /// Note currently played by clicking the on-screen piano.
    mouse_note: Option<u8>,
    mixer_panel: MixerPanel,
}

impl App {
    pub fn new(params: AppParams) -> Self {
        Self {
            params,
            active: 0,
            held_keys: BTreeSet::new(),
            mouse_note: None,
            mixer_panel: MixerPanel::default(),
        }
    }

    /// Sends a note to the active instrument.
    fn send(&mut self, event: NoteEvent) {
        if let Some(instrument) = self.params.instruments.get_mut(self.active) {
            let _ = instrument.notes.push(event);
        }
    }

    /// Asks for a WAV file and adds a sampler that plays it.
    fn add_sample(&mut self) {
        let mut dialog = rfd::FileDialog::new()
            .set_title("Add sample")
            .add_filter("WAV audio", &["wav"]);
        if let Ok(dir) = std::fs::canonicalize("testsamples") {
            dialog = dialog.set_directory(dir);
        }
        let Some(path) = dialog.pick_file() else {
            return;
        };

        match self.params.add_sample(&path) {
            Ok(Some(index)) => self.set_active(index),
            Ok(None) => eprintln!("can't add more instruments"),
            Err(err) => eprintln!("failed to load {}: {err}", path.display()),
        }
    }

    /// Switches the keyboard to another instrument. Releases every held note
    /// on the old one first, otherwise those notes would never get a note off.
    fn set_active(&mut self, index: usize) {
        let held: Vec<u8> = self.held_keys.iter().copied().chain(self.mouse_note).collect();
        for n in held {
            self.send(NoteEvent::Off(n));
        }
        self.held_keys.clear();
        self.mouse_note = None;

        self.active = index;
    }

    fn handle_keys(&mut self, ui: &egui::Ui) {
        let events: Vec<NoteEvent> =
            ui.input(|i| i.events.iter().filter_map(key_to_note_event).collect());

        for note_event in events {
            match note_event {
                NoteEvent::On(n) => {
                    self.held_keys.insert(n);
                }
                NoteEvent::Off(n) => {
                    self.held_keys.remove(&n);
                }
            }

            self.send(note_event);
        }
    }

    fn set_mouse_note(&mut self, note: Option<u8>) {
        if note == self.mouse_note {
            return;
        }
        if let Some(n) = self.mouse_note {
            self.send(NoteEvent::Off(n));
        }
        if let Some(n) = note {
            self.send(NoteEvent::On(n));
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
        self.params.collect_garbage();
        self.handle_keys(ui);

        egui::Panel::top("header")
            .frame(theme::panel_frame())
            .resizable(false)
            .show(ui, |ui| self.header(ui));

        // bottom panels stack upwards: the piano at the very bottom, the mixer above it
        egui::Panel::bottom("keyboard")
            .frame(theme::panel_frame())
            .resizable(false)
            .show(ui, |ui| self.keyboard(ui));

        egui::Panel::bottom("mixer")
            .frame(theme::panel_frame())
            .resizable(false)
            .show(ui, |ui| self.mixer_panel.show(ui, &mut self.params));

        let mut instrument_action = None;
        egui::CentralPanel::default()
            .frame(egui::Frame::new().inner_margin(16))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(theme::GAP, theme::GAP);
                ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                    instrument_action = instrument_panel::show(ui, &self.params, self.active);
                    editor_panel::show(ui, &mut self.params, self.active);
                });
            });

        // handled after drawing, so the file dialog doesn't open mid-frame
        match instrument_action {
            Some(InstrumentAction::Select(index)) => self.set_active(index),
            Some(InstrumentAction::AddSynth) => {
                if let Some(index) = self.params.add_synth() {
                    self.set_active(index);
                }
            }
            Some(InstrumentAction::AddSample) => self.add_sample(),
            None => {}
        }

        // keep redrawing so the meters move without mouse input
        ui.ctx().request_repaint();
    }
}
