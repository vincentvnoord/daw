mod actions;

use std::time::Instant;

use eframe::egui::{self, RichText};

use crate::{
    audio::latency,
    project::{InputDevice, Project},
    ui::{
        editor, keyboard::Keyboard, mixer::MixerPanel, piano_roll::PianoRoll, theme, tracks,
        tracks::timeline::Timeline, transport,
    },
};

pub struct App {
    project: Project,
    /// Index of the selected track, the one the keyboard plays.
    active: usize,
    keyboard: Keyboard,
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
            keyboard: Keyboard::default(),
            mixer_panel: MixerPanel::default(),
            input_devices: Vec::new(),
            timeline: Timeline::default(),
            piano_roll: None,
            latency_logged: Instant::now(),
        }
    }

    /// Switches the keyboard to another track, letting go of whatever it
    /// held on the old one.
    fn set_active(&mut self, index: usize) {
        self.keyboard.release_all(&mut self.project, self.active);
        self.active = index;
    }

    fn header(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let (dot, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
            ui.painter().circle_filled(dot.center(), 5.0, theme::ACCENT);
            ui.label(RichText::new("Vincent Studio").heading().strong());
            ui.add_space(theme::GAP * 2.0);
            transport::show(ui, &self.project);
        });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.project.collect_garbage();
        if latency::logging() && self.latency_logged.elapsed().as_secs() >= 1 {
            self.latency_logged = Instant::now();
            self.project.log_latency();
        }
        self.keyboard
            .handle_keys(ui, &mut self.project, self.active);

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
            .show(ui, |ui| {
                self.keyboard.show(ui, &mut self.project, self.active)
            });

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
                track_action = tracks::show(ui, &self.project, self.active, &mut self.timeline);
                ui.add_space(theme::GAP);
                match &mut self.piano_roll {
                    Some(roll) => {
                        if !roll.show(ui, &mut self.project) {
                            self.piano_roll = None;
                        }
                    }
                    None => {
                        add_clip = editor::show(
                            ui,
                            &mut self.project,
                            self.active,
                            &mut self.input_devices,
                        );
                    }
                }
            });

        // handled after drawing, so a file dialog doesn't open mid-frame
        if add_clip {
            self.add_clip();
        }
        if let Some(action) = track_action {
            self.apply(action);
        }

        // keep redrawing so the meters move without mouse input
        ui.ctx().request_repaint();
    }
}
