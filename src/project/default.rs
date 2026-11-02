use std::path::Path;

use crate::{
    audio::{note::NoteEvent, schedule::ScheduledNote, studio::Studio},
    project::Project,
};

/// Found from the crate root, so the app finds it whatever directory it's
/// started from.
const KICK_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/testsamples/kick_16bit_stereo_44k.wav"
);

/// How long the test pattern runs before it goes quiet; there's no looping
/// yet.
const PATTERN_BARS: usize = 8;

/// The project the app starts with: a synth track and a kick track, each on
/// a channel of its own, playing a test pattern. Built the same way the UI
/// adds tracks, so the studio picks them up from the command queue on its
/// first buffer.
///
/// Returns the project for the UI and the studio for the audio engine.
pub fn default_project(sample_rate: f32) -> (Project, Studio) {
    let (mut project, studio) = Project::new(sample_rate);

    let synth = project.add_synth_track();
    // the kick is a nice-to-have, so the app still starts without it
    let kick = match project.add_named_sample_track(Path::new(KICK_PATH), "Kick") {
        Ok(kick) => kick,
        Err(err) => {
            eprintln!("failed to load {KICK_PATH}: {err}");
            None
        }
    };

    project.set_schedule(test_pattern(synth, kick));

    (project, studio)
}

/// A kick on every beat and a bass line on the synth, to hear the schedule
/// play in time with the metronome. Stands in until tracks have sequences.
/// Track indices are instrument slots, since tracks are never removed yet.
fn test_pattern(synth: Option<usize>, kick: Option<usize>) -> Vec<ScheduledNote> {
    // root notes of a four-bar progression: C, A, F, G
    const ROOTS: [u8; 4] = [36, 45, 41, 43];

    let mut notes = Vec::new();
    let mut note = |instrument: usize, start: f64, length: f64, pitch: u8| {
        notes.push(ScheduledNote {
            beat: start,
            instrument,
            event: NoteEvent::On(pitch),
        });
        notes.push(ScheduledNote {
            beat: start + length,
            instrument,
            event: NoteEvent::Off(pitch),
        });
    };

    for bar in 0..PATTERN_BARS {
        let start = bar as f64 * 4.0;

        if let Some(kick) = kick {
            for beat in 0..4 {
                note(kick, start + beat as f64, 0.5, 60);
            }
        }

        if let Some(synth) = synth {
            // root on the beat, octave up on the off-beat eighths
            let root = ROOTS[bar % ROOTS.len()];
            for eighth in 0..8 {
                let pitch = if eighth % 2 == 0 { root } else { root + 12 };
                note(synth, start + eighth as f64 * 0.5, 0.4, pitch);
            }
        }
    }

    notes
}
