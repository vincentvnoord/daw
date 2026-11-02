use std::path::Path;

use crate::{
    audio::{studio::Studio, transport::metronome::BEATS_PER_BAR},
    project::{Note, Placement, Project, Sequence},
};

/// Found from the crate root, so the app finds it whatever directory it's
/// started from.
const KICK_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/testsamples/kick_16bit_stereo_44k.wav"
);

/// How long the demo song runs before it goes quiet; there's no looping yet.
const SONG_BARS: usize = 8;

/// The project the app starts with: a synth track and a kick track, each on
/// a channel of its own, playing a short demo song. Built the same way the
/// UI adds tracks, so the studio picks them up from the command queue on its
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

    if let Some(synth) = synth {
        let bass = project.add_sequence(bass_line());
        // four bars long, so twice to fill the song
        place_every(&mut project, synth, bass, 4);
    }
    if let Some(kick) = kick {
        let four_on_the_floor = project.add_sequence(kick_pattern());
        place_every(&mut project, kick, four_on_the_floor, 1);
    }
    project.update_schedule();

    (project, studio)
}

/// Places `sequence` on `track` every `bars` bars, through the whole song.
fn place_every(project: &mut Project, track: usize, sequence: usize, bars: usize) {
    for bar in (0..SONG_BARS).step_by(bars) {
        project.tracks[track].placements.push(Placement {
            start: bar as f64 * BEATS_PER_BAR as f64,
            sequence,
        });
    }
}

/// One bar: a kick on every beat.
fn kick_pattern() -> Sequence {
    Sequence {
        name: "Four on the floor".to_owned(),
        length: BEATS_PER_BAR as f64,
        notes: (0..4)
            .map(|beat| Note {
                start: beat as f64,
                length: 0.5,
                pitch: 60,
            })
            .collect(),
    }
}

/// Four bars of eighth notes over C, A, F, G: the root on the beat, an
/// octave up on the off-beat.
fn bass_line() -> Sequence {
    const ROOTS: [u8; 4] = [36, 45, 41, 43];

    let notes = ROOTS
        .iter()
        .enumerate()
        .flat_map(|(bar, &root)| {
            (0..8).map(move |eighth| Note {
                start: bar as f64 * BEATS_PER_BAR as f64 + eighth as f64 * 0.5,
                length: 0.4,
                pitch: if eighth % 2 == 0 { root } else { root + 12 },
            })
        })
        .collect();

    Sequence {
        name: "Bass".to_owned(),
        length: ROOTS.len() as f64 * BEATS_PER_BAR as f64,
        notes,
    }
}
