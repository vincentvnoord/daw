//! What the app does with what the user did in the panels, once the frame
//! is drawn.

use std::{path::PathBuf, sync::atomic::Ordering};

use crate::{
    audio::transport::beats_at,
    ui::{app::App, piano_roll::PianoRoll, tracks::TrackAction},
};

impl App {
    /// Carries out what the user did in the track list or on the timeline.
    pub(super) fn apply(&mut self, action: TrackAction) {
        match action {
            TrackAction::Select(index) => self.set_active(index),
            TrackAction::AddSynth => {
                if let Some(index) = self.project.add_synth_track() {
                    self.set_active(index);
                }
            }
            TrackAction::AddSample => self.add_sample(),
            TrackAction::AddAudio => {
                if let Some(index) = self.project.add_audio_track() {
                    self.set_active(index);
                }
            }
            TrackAction::MovePlacement {
                from,
                index,
                to,
                start,
            } => self.project.move_placement(from, index, to, start),
            TrackAction::MoveClip {
                from,
                index,
                to: Some(to),
                start,
            } => self.project.move_clip(from, index, to, start),
            // dropped on the empty track: it becomes an audio track first
            TrackAction::MoveClip {
                from,
                index,
                to: None,
                start,
            } => {
                if let Some(to) = self.project.add_audio_track() {
                    self.project.move_clip(from, index, to, start);
                    self.set_active(to);
                }
            }
            TrackAction::OpenSequence { track, sequence } => {
                self.set_active(track);
                self.piano_roll = Some(PianoRoll::new(sequence, track));
            }
        }
    }

    /// Asks for a WAV file and places it on the selected audio track, at the
    /// beat the playhead is in.
    pub(super) fn add_clip(&mut self) {
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
