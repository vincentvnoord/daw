use std::path::Path;

use crate::{audio::studio::Studio, project::Project};

/// Found from the crate root, so the app finds it whatever directory it's
/// started from.
const KICK_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/testsamples/kick_16bit_stereo_44k.wav"
);

/// The project the app starts with: a synth track and a kick track, each on
/// a channel of its own. Built the same way the UI adds tracks, so the studio
/// picks them up from the command queue on its first buffer.
///
/// Returns the project for the UI and the studio for the audio engine.
pub fn default_project(sample_rate: f32) -> (Project, Studio) {
    let (mut project, studio) = Project::new(sample_rate);

    project.add_synth_track();
    // the kick is a nice-to-have, so the app still starts without it
    if let Err(err) = project.add_named_sample_track(Path::new(KICK_PATH), "Kick") {
        eprintln!("failed to load {KICK_PATH}: {err}");
    }

    (project, studio)
}
