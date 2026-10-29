use std::{path::Path, sync::Arc};

use rtrb::Producer;

use crate::{
    audio::{
        mixer::{MixerParams, meter::Meter},
        sampler::{Sampler, sample::Sample},
        studio::studio::Studio,
        synth::Synth,
    },
    events::NoteEvent,
    helpers::AtomicF32,
    shared::AppParams,
};

/// The setup the app starts with: a kick on a "Drums" channel and a synth on
/// a "Keys" channel.
///
/// Returns the studio for the audio engine, the params for the UI, and the
/// note input the keyboard plays (the synth).
pub fn default_studio(sample_rate: f32) -> (Studio, AppParams, Producer<NoteEvent>) {
    let mixer = Arc::new(MixerParams {
        gain: AtomicF32::new(0.5),
        meter: Meter::new(),
    });
    let mut studio = Studio::new(Arc::clone(&mixer));

    let (drums, drums_params) = studio.add_channel(0.5);
    let (keys, keys_params) = studio.add_channel(0.5);

    let kick = Sample::load(Path::new("testsamples/kick_16bit_stereo_44k.wav"), sample_rate)
        .expect("failed to load kick sample");
    let (_kick_notes, _) = studio.add_instrument(Sampler::new(kick.data, 8), Some(drums));

    let synth = Synth::default(sample_rate);
    let synth_params = synth.params();
    let (synth_notes, _) = studio.add_instrument(synth, Some(keys));

    let params = AppParams {
        mixer,
        channels: vec![
            ("Drums".to_owned(), drums_params),
            ("Keys".to_owned(), keys_params),
        ],
        synth: synth_params,
    };

    (studio, params, synth_notes)
}
