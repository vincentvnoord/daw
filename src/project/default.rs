use std::path::Path;

use crate::{
    audio::{studio::Studio, synth::osc::WaveForm, transport::metronome::BEATS_PER_BAR},
    project::{InstrumentKind, Note, Placement, Project, Sequence},
};

/// Found from the crate root, so the app finds them whatever directory it's
/// started from.
const KICK_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/testsamples/kick_16bit_stereo_44k.wav"
);
const SNARE_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/testsamples/snare_24bit_mono_48k.wav"
);
const HATS_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/testsamples/hihat_32float_stereo_48k.wav"
);

/// Fast enough for sixteenth-note plucks and hat rolls, with the drums in
/// half time so it still feels slow.
const SONG_BPM: f32 = 150.0;
/// How long the demo song runs; the loop plays it round and round.
const SONG_BARS: usize = 8;
/// The bars the chords take to come round, and the length of every
/// sequence that follows them.
const PHRASE_BARS: usize = 4;

/// The chords, one bar each: Amaj9, Badd9, G#m7, C#m9 in E major, voiced
/// without their roots (the bass has those) and all keeping B on top, so they
/// move as little as possible.
const CHORDS: [[u8; 4]; PHRASE_BARS] = [
    [61, 64, 68, 71],
    [61, 63, 66, 71],
    [63, 66, 68, 71],
    [63, 64, 68, 71],
];
/// The chords' roots, A B G# C#, for the bass.
const ROOTS: [u8; PHRASE_BARS] = [45, 47, 44, 49];

/// How a synth track is set up: the waveform, the filter, and the envelope
/// in seconds (sustain is a level).
struct Sound {
    waveform: WaveForm,
    cutoff: f32,
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
    /// The fader of the track's channel.
    gain: f32,
}

/// The project the app starts with: a short demo song of chords, a plucked
/// arpeggio, a lead, a bass and drums, each track on a channel of its own,
/// looping. Built the same way the UI adds tracks, so the studio picks them
/// up from the command queue on its first buffer.
///
/// Returns the project for the UI and the studio for the audio engine.
pub fn default_project(sample_rate: f32) -> (Project, Studio) {
    let (mut project, studio) = Project::new(sample_rate);
    project.transport.bpm.set(SONG_BPM);
    project.transport.loop_area.set(0.0, bar_start(SONG_BARS));

    let chords = add_synth(
        &mut project,
        "Chords",
        Sound {
            waveform: WaveForm::Saw,
            cutoff: 900.0,
            attack: 0.08,
            decay: 0.6,
            sustain: 0.6,
            release: 0.8,
            gain: 0.35,
        },
    );
    let pluck = add_synth(
        &mut project,
        "Pluck",
        Sound {
            waveform: WaveForm::Saw,
            cutoff: 2500.0,
            attack: 0.002,
            decay: 0.22,
            sustain: 0.0,
            release: 0.2,
            gain: 0.5,
        },
    );
    let lead = add_synth(
        &mut project,
        "Lead",
        Sound {
            waveform: WaveForm::Square,
            cutoff: 1800.0,
            attack: 0.01,
            decay: 0.3,
            sustain: 0.7,
            release: 0.3,
            gain: 0.4,
        },
    );
    let bass = add_synth(
        &mut project,
        "Bass",
        Sound {
            waveform: WaveForm::Sine,
            cutoff: 2000.0,
            attack: 0.005,
            decay: 0.3,
            sustain: 0.9,
            release: 0.15,
            gain: 0.9,
        },
    );
    // the drums are a nice-to-have, so the app still starts without them
    let kick = add_sample(&mut project, KICK_PATH, "Kick");
    let snare = add_sample(&mut project, SNARE_PATH, "Snare");
    let hats = add_sample(&mut project, HATS_PATH, "Hats");

    let parts = [
        (chords, chord_pad(), 0, PHRASE_BARS),
        (pluck, pluck_arp(), 0, PHRASE_BARS),
        // the lead comes in the second time round
        (lead, lead_melody(), PHRASE_BARS, PHRASE_BARS),
        (bass, bass_line(), 0, PHRASE_BARS),
        (kick, drum_bar("Kick", &[0.0, 1.5, 2.75]), 0, 1),
        // half time: the snare on the third beat, not the second and fourth
        (snare, drum_bar("Snare", &[2.0]), 0, 1),
        (hats, hat_pattern(), 0, PHRASE_BARS),
    ];
    for (track, sequence, from, every) in parts {
        if let Some(track) = track {
            let sequence = project.add_sequence(sequence);
            place_every(&mut project, track, sequence, from, every);
        }
    }
    project.update_schedule();

    (project, studio)
}

/// Adds a synth track named `name` and sets it up as `sound`.
fn add_synth(project: &mut Project, name: &str, sound: Sound) -> Option<usize> {
    let track = project.add_named_synth_track(name)?;
    if let Some(InstrumentKind::Synth(params)) = project.tracks[track].instrument().map(|i| &i.kind)
    {
        params.waveform.set(sound.waveform);
        params.cutoff.set(sound.cutoff);
        params.attack.set(sound.attack);
        params.decay.set(sound.decay);
        params.sustain.set(sound.sustain);
        params.release.set(sound.release);
    }
    set_gain(project, track, sound.gain);
    Some(track)
}

/// Adds a sampler track playing the sample at `path`, or reports why it
/// couldn't.
fn add_sample(project: &mut Project, path: &str, name: &str) -> Option<usize> {
    match project.add_named_sample_track(Path::new(path), name) {
        Ok(track) => track,
        Err(err) => {
            eprintln!("failed to load {path}: {err}");
            None
        }
    }
}

/// Sets the fader of the channel `track` plays into.
fn set_gain(project: &Project, track: usize, gain: f32) {
    let id = project.tracks[track].route().get_channel();
    if let Some(channel) = project.channels.iter().find(|c| Some(c.id()) == id) {
        channel.params.gain.set(gain);
    }
}

/// Places `sequence` on `track` every `bars` bars, from bar `from` to the end
/// of the song.
fn place_every(project: &mut Project, track: usize, sequence: usize, from: usize, bars: usize) {
    for bar in (from..SONG_BARS).step_by(bars) {
        project.tracks[track].placements.push(Placement {
            start: bar_start(bar),
            sequence,
        });
    }
}

/// The beat bar `bar` starts on.
fn bar_start(bar: usize) -> f64 {
    bar as f64 * BEATS_PER_BAR as f64
}

/// A sequence one phrase long.
fn phrase(name: &str, notes: Vec<Note>) -> Sequence {
    Sequence {
        name: name.to_owned(),
        length: bar_start(PHRASE_BARS),
        notes,
    }
}

/// The chords, held for a bar each.
fn chord_pad() -> Sequence {
    let notes = CHORDS
        .iter()
        .enumerate()
        .flat_map(|(bar, chord)| {
            chord.iter().map(move |&pitch| Note {
                start: bar_start(bar),
                // let go just before the next chord, so its release overlaps
                // the next attack instead of the voices being stolen
                length: BEATS_PER_BAR as f64 - 0.1,
                pitch,
            })
        })
        .collect();

    phrase("Chords", notes)
}

/// Sixteenths running through each chord an octave up, with the top note
/// doubled an octave above that, like a picked guitar.
fn pluck_arp() -> Sequence {
    // into the chord's notes, the fifth being the first an octave up; `None`
    // is a rest, so the pattern breathes at the end of the bar
    const PATTERN: [Option<usize>; 16] = [
        Some(0),
        Some(1),
        Some(2),
        Some(3),
        Some(4),
        Some(3),
        Some(2),
        Some(1),
        Some(0),
        Some(2),
        Some(4),
        Some(2),
        Some(3),
        Some(1),
        Some(2),
        None,
    ];

    let notes = CHORDS
        .iter()
        .enumerate()
        .flat_map(|(bar, chord)| {
            let tones = [chord[0], chord[1], chord[2], chord[3], chord[0] + 12];
            PATTERN.iter().enumerate().filter_map(move |(step, index)| {
                index.map(|i| Note {
                    start: bar_start(bar) + step as f64 * 0.25,
                    length: 0.2,
                    pitch: tones[i] + 12,
                })
            })
        })
        .collect();

    phrase("Pluck", notes)
}

/// A melody over the four chords, mostly in E major pentatonic, landing on a
/// chord note at the start of every bar.
fn lead_melody() -> Sequence {
    // (bar, beat, length, pitch)
    const MELODY: [(usize, f64, f64, u8); 19] = [
        (0, 0.0, 0.7, 71),
        (0, 0.75, 0.7, 73),
        (0, 1.5, 0.9, 76),
        (0, 2.5, 0.45, 73),
        (0, 3.0, 0.9, 71),
        (1, 0.0, 0.7, 75),
        (1, 0.75, 0.7, 73),
        (1, 1.5, 1.4, 71),
        (1, 3.0, 0.45, 66),
        (1, 3.5, 0.45, 68),
        (2, 0.0, 0.7, 71),
        (2, 0.75, 0.7, 75),
        (2, 1.5, 0.9, 78),
        (2, 2.5, 0.45, 76),
        (2, 3.0, 0.9, 75),
        (3, 0.0, 1.4, 76),
        (3, 1.5, 0.45, 75),
        (3, 2.0, 0.9, 71),
        (3, 3.0, 0.9, 68),
    ];

    let notes = MELODY
        .iter()
        .map(|&(bar, beat, length, pitch)| Note {
            start: bar_start(bar) + beat,
            length,
            pitch,
        })
        .collect();

    phrase("Lead", notes)
}

/// The roots in an 808 rhythm that follows the kick: a long note on the one,
/// then two short ones pushing into the next bar.
fn bass_line() -> Sequence {
    let notes = ROOTS
        .iter()
        .enumerate()
        .flat_map(|(bar, &root)| {
            [(0.0, 1.4), (1.5, 1.1), (2.75, 1.2)]
                .into_iter()
                .map(move |(beat, length)| Note {
                    start: bar_start(bar) + beat,
                    length,
                    pitch: root,
                })
        })
        .collect();

    phrase("Bass", notes)
}

/// One bar of a drum hit on each of `beats`.
fn drum_bar(name: &str, beats: &[f64]) -> Sequence {
    Sequence {
        name: name.to_owned(),
        length: BEATS_PER_BAR as f64,
        notes: beats.iter().map(|&beat| hit(beat)).collect(),
    }
}

/// Eighths, with a sixteenth pickup every other bar and a triplet roll into
/// the top of the phrase.
fn hat_pattern() -> Sequence {
    let mut notes = Vec::new();
    for bar in 0..PHRASE_BARS {
        let start = bar_start(bar);
        let last_bar = bar == PHRASE_BARS - 1;

        for eighth in 0..8 {
            let beat = eighth as f64 * 0.5;
            // the last beat of the phrase is the roll instead
            if !(last_bar && beat >= 3.0) {
                notes.push(hit(start + beat));
            }
        }
        if bar % 2 == 1 {
            notes.push(hit(start + 1.75));
        }
        if last_bar {
            notes.extend((0..6).map(|i| hit(start + 3.0 + i as f64 / 6.0)));
        }
    }

    phrase("Hats", notes)
}

/// A drum hit: samplers play the whole sample whatever the length.
fn hit(start: f64) -> Note {
    Note {
        start,
        length: 0.1,
        pitch: 60,
    }
}
