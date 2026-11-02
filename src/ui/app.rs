use std::collections::BTreeSet;

use eframe::egui::{self, RichText};

use crate::{
    audio::note::NoteEvent,
    project::Project,
    ui::{
        editor_panel,
        input_panel::InputPanel,
        keyboard::{self, key_to_note_event},
        mixer_panel::MixerPanel,
        theme,
        track_panel::{self, TrackAction},
        transport_panel,
        widgets::piano::piano,
    },
};

/// Width of the column with the input list.
const LIST_WIDTH: f32 = 200.0;

pub struct App {
    project: Project,
    /// Index of the selected track, the one the keyboard plays.
    active: usize,
    held_keys: BTreeSet<u8>,
    /// Note currently played by clicking the on-screen piano.
    mouse_note: Option<u8>,
    mixer_panel: MixerPanel,
    input_panel: InputPanel,
}

impl App {
    pub fn new(project: Project) -> Self {
        Self {
            project,
            active: 0,
            held_keys: BTreeSet::new(),
            mouse_note: None,
            mixer_panel: MixerPanel::default(),
            input_panel: InputPanel::default(),
        }
    }

    /// Sends a note to the selected track's instrument.
    fn send(&mut self, event: NoteEvent) {
        if let Some(track) = self.project.tracks.get_mut(self.active) {
            let _ = track.instrument.notes.push(event);
        }
    }

    /// Asks for a WAV file and adds a track with a sampler that plays it.
    fn add_sample(&mut self) {
        let mut dialog = rfd::FileDialog::new()
            .set_title("Add sample")
            .add_filter("WAV audio", &["wav"]);
        if let Ok(dir) = std::fs::canonicalize(concat!(env!("CARGO_MANIFEST_DIR"), "/testsamples"))
        {
            dialog = dialog.set_directory(dir);
        }
        let Some(path) = dialog.pick_file() else {
            return;
        };

        match self.project.add_sample_track(&path) {
            Ok(Some(index)) => self.set_active(index),
            Ok(None) => eprintln!("can't add more tracks"),
            Err(err) => eprintln!("failed to load {}: {err}", path.display()),
        }
    }

    /// Switches the keyboard to another track. Releases every held note
    /// on the old one first, otherwise those notes would never get a note off.
    fn set_active(&mut self, index: usize) {
        let held: Vec<u8> = self
            .held_keys
            .iter()
            .copied()
            .chain(self.mouse_note)
            .collect();
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
            ui.add_space(theme::GAP * 2.0);
            transport_panel::show(ui, &self.project);
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
        self.project.collect_garbage();
        self.handle_keys(ui);

        // not while typing in a field, like the tempo. Consumed, so a focused
        // button doesn't take the same press as a click.
        if !ui.ctx().egui_wants_keyboard_input()
            && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Space))
        {
            self.project.transport.toggle_playing();
        }

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
            .show(ui, |ui| self.mixer_panel.show(ui, &mut self.project));

        // inputs aren't tracks (yet), so they sit to the side, out of the timeline
        let mut new_input = None;
        egui::Panel::right("inputs")
            .frame(theme::panel_frame())
            .resizable(false)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(theme::GAP, theme::GAP);
                ui.set_width(LIST_WIDTH);
                new_input = self.input_panel.show(ui, &self.project);
            });

        let mut track_action = None;
        egui::CentralPanel::default()
            .frame(egui::Frame::new().inner_margin(16))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(theme::GAP, theme::GAP);
                track_action = track_panel::show(ui, &self.project, self.active);
                ui.add_space(theme::GAP);
                editor_panel::show(ui, &mut self.project, self.active);
            });

        if let Some(device) = new_input {
            match self.project.add_input(&device) {
                Ok(Some(_)) => {}
                Ok(None) => eprintln!("can't add more inputs"),
                Err(err) => eprintln!("failed to open {}: {err}", device.name),
            }
        }

        // handled after drawing, so the file dialog doesn't open mid-frame
        match track_action {
            Some(TrackAction::Select(index)) => self.set_active(index),
            Some(TrackAction::AddSynth) => {
                if let Some(index) = self.project.add_synth_track() {
                    self.set_active(index);
                }
            }
            Some(TrackAction::AddSample) => self.add_sample(),
            None => {}
        }

        // keep redrawing so the meters move without mouse input
        ui.ctx().request_repaint();
    }
}
