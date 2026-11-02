use std::{collections::BTreeSet, path::PathBuf, sync::atomic::Ordering, time::Instant};

use eframe::egui::{self, RichText};

use crate::{
    audio::{latency, note::NoteEvent, transport::beats_at},
    project::{InputDevice, Project, Track},
    ui::{
        editor_panel,
        keyboard::{self, key_to_note_event},
        mixer_panel::MixerPanel,
        piano_roll::PianoRoll,
        theme,
        timeline::Timeline,
        track_panel::{self, TrackAction},
        transport_panel,
        widgets::piano::piano,
    },
};

pub struct App {
    project: Project,
    /// Index of the selected track, the one the keyboard plays.
    active: usize,
    held_keys: BTreeSet<u8>,
    /// Note currently played by clicking the on-screen piano.
    mouse_note: Option<u8>,
    mixer_panel: MixerPanel,
    /// Recording devices offered for an audio track's input, looked up when
    /// the list opens, since asking the system every frame would be slow.
    input_devices: Vec<InputDevice>,
    timeline: Timeline,
    /// The sequence open for editing, shown instead of the instrument editor.
    piano_roll: Option<PianoRoll>,
    /// When the latency stats were last printed.
    latency_logged: Instant,
}

impl App {
    pub fn new(project: Project) -> Self {
        Self {
            project,
            active: 0,
            held_keys: BTreeSet::new(),
            mouse_note: None,
            mixer_panel: MixerPanel::default(),
            input_devices: Vec::new(),
            timeline: Timeline::default(),
            piano_roll: None,
            latency_logged: Instant::now(),
        }
    }

    /// Sends a note to the selected track's instrument.
    fn send(&mut self, event: NoteEvent) {
        if let Some(instrument) = self
            .project
            .tracks
            .get_mut(self.active)
            .and_then(Track::instrument_mut)
        {
            let _ = instrument.notes.push(event);
        }
    }

    /// Asks for a WAV file and adds a track with a sampler that plays it.
    fn add_sample(&mut self) {
        let Some(path) = pick_wav("Add sample") else {
            return;
        };

        match self.project.add_sample_track(&path) {
            Ok(Some(index)) => self.set_active(index),
            Ok(None) => eprintln!("can't add more tracks"),
            Err(err) => eprintln!("failed to load {}: {err}", path.display()),
        }
    }

    /// Asks for a WAV file and places it on the selected audio track, at the
    /// beat the playhead is in.
    fn add_clip(&mut self) {
        let Some(path) = pick_wav("Add clip") else {
            return;
        };

        let samples = self.project.transport.position.load(Ordering::Relaxed);
        let bpm = self.project.transport.bpm.get() as f64;
        let start = beats_at(samples, bpm, self.project.sample_rate as f64).floor();
        if let Err(err) = self.project.add_clip(self.active, &path, start) {
            eprintln!("failed to load {}: {err}", path.display());
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
        if latency::logging() && self.latency_logged.elapsed().as_secs() >= 1 {
            self.latency_logged = Instant::now();
            self.project.log_latency();
        }
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

        let mut track_action = None;
        let mut add_clip = false;
        egui::CentralPanel::default()
            .frame(egui::Frame::new().inner_margin(16))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(theme::GAP, theme::GAP);
                track_action =
                    track_panel::show(ui, &self.project, self.active, &mut self.timeline);
                ui.add_space(theme::GAP);
                match &mut self.piano_roll {
                    Some(roll) => {
                        if !roll.show(ui, &mut self.project) {
                            self.piano_roll = None;
                        }
                    }
                    None => {
                        add_clip = editor_panel::show(
                            ui,
                            &mut self.project,
                            self.active,
                            &mut self.input_devices,
                        );
                    }
                }
            });

        if add_clip {
            self.add_clip();
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
            Some(TrackAction::AddAudio) => {
                if let Some(index) = self.project.add_audio_track() {
                    self.set_active(index);
                }
            }
            Some(TrackAction::MovePlacement {
                from,
                index,
                to,
                start,
            }) => self.project.move_placement(from, index, to, start),
            Some(TrackAction::MoveClip {
                from,
                index,
                to: Some(to),
                start,
            }) => self.project.move_clip(from, index, to, start),
            // dropped on the empty track: it becomes an audio track first
            Some(TrackAction::MoveClip {
                from,
                index,
                to: None,
                start,
            }) => {
                if let Some(to) = self.project.add_audio_track() {
                    self.project.move_clip(from, index, to, start);
                    self.set_active(to);
                }
            }
            Some(TrackAction::OpenSequence { track, sequence }) => {
                self.set_active(track);
                self.piano_roll = Some(PianoRoll::new(sequence, track));
            }
            None => {}
        }

        // keep redrawing so the meters move without mouse input
        ui.ctx().request_repaint();
    }
}

/// Asks for a WAV file, starting in the test samples folder when there is
/// one. `None` when the dialog is cancelled.
fn pick_wav(title: &str) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new()
        .set_title(title)
        .add_filter("WAV audio", &["wav"]);
    if let Ok(dir) = std::fs::canonicalize(concat!(env!("CARGO_MANIFEST_DIR"), "/testsamples")) {
        dialog = dialog.set_directory(dir);
    }
    dialog.pick_file()
}
