mod actions;

use std::time::Instant;

use eframe::egui::{self, RichText};

use crate::{
    audio::latency,
    project::{InputDevice, Project},
    ui::{
        app::actions::Action, editor, keyboard::Keyboard, mixer::MixerPanel, piano_roll::PianoRoll,
        theme, tracks, tracks::timeline::Timeline, transport, widgets::sidebar::sidebar,
    },
};

/// Room for the editor column: the amp envelope's four knobs in a row, plus
/// the padding of the editor frame and the card around them.
const EDITOR_WIDTH: f32 = 360.0;

/// What fills the main area, right of the editor. Opening a sequence goes a
/// level deeper, in place of the tracks; closing it comes back up.
enum View {
    Tracks,
    PianoRoll(PianoRoll),
}

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
    view: View,
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
            view: View::Tracks,
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

    /// The selected track's editor on the left, and the view beside it.
    fn workspace(&mut self, ui: &mut egui::Ui) -> [Option<Action>; 2] {
        ui.spacing_mut().item_spacing = egui::vec2(theme::GAP, theme::GAP);
        let (side, main) = sidebar(ui, EDITOR_WIDTH, self, Self::editor, Self::view);
        [side, main]
    }

    fn editor(&mut self, ui: &mut egui::Ui) -> Option<Action> {
        let add_clip = editor::show(ui, &mut self.project, self.active, &mut self.input_devices);
        add_clip.then_some(Action::AddClip)
    }

    fn view(&mut self, ui: &mut egui::Ui) -> Option<Action> {
        match &mut self.view {
            View::Tracks => {
                tracks::show(ui, &self.project, self.active, &mut self.timeline).map(Action::Track)
            }
            View::PianoRoll(roll) => {
                let open = roll.show(ui, &mut self.project);
                (!open).then_some(Action::ShowTracks)
            }
        }
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

        egui::Panel::bottom("mixer")
            .frame(theme::panel_frame())
            .resizable(false)
            .show(ui, |ui| self.mixer_panel.show(ui, &mut self.project));

        let actions = egui::CentralPanel::default()
            .frame(egui::Frame::new().inner_margin(16))
            .show(ui, |ui| self.workspace(ui))
            .inner;

        // carried out after drawing, so a file dialog doesn't open mid-frame
        for action in actions.into_iter().flatten() {
            self.apply(action);
        }

        // keep redrawing so the meters move without mouse input
        ui.ctx().request_repaint();
    }
}
