use std::{path::Path, sync::Arc};

use rtrb::RingBuffer;

use crate::{
    audio::{
        mixer::{MixerParams, channel::DEFAULT_GAIN, meter::Meter},
        sampler::{Sampler, VOICES, sample::Sample},
        studio::studio::Studio,
        synth::Synth,
        transport::{TransportParams, metronome::MetronomeParams},
    },
    helpers::AtomicF32,
    shared::{AppParams, ChannelHandle, InstrumentHandle, InstrumentKind},
};

/// How many changes from the UI can wait for the audio thread. The garbage
/// queue is the same size, so everything a batch of commands removes fits.
const COMMAND_QUEUE_SIZE: usize = 64;

const KICK_PATH: &str = "testsamples/kick_16bit_stereo_44k.wav";

/// The setup the app starts with: a kick on a "Drums" channel and a synth on
/// a "Keys" channel.
///
/// Returns the studio for the audio engine and the params for the UI.
pub fn default_studio(sample_rate: f32) -> (Studio, AppParams) {
    let mixer = Arc::new(MixerParams {
        gain: AtomicF32::new(0.5),
        meter: Meter::new(),
    });
    let transport = Arc::new(TransportParams::new());
    let metronome = Arc::new(MetronomeParams::new());
    let (commands, command_queue) = RingBuffer::new(COMMAND_QUEUE_SIZE);
    let (garbage_bin, garbage) = RingBuffer::new(COMMAND_QUEUE_SIZE);
    let mut studio = Studio::new(
        Arc::clone(&mixer),
        Arc::clone(&transport),
        Arc::clone(&metronome),
        command_queue,
        garbage_bin,
        sample_rate,
    );

    let drums = studio.add_channel(DEFAULT_GAIN);
    let keys = studio.add_channel(DEFAULT_GAIN);

    let kick = Sample::load(Path::new(KICK_PATH), sample_rate).expect("failed to load kick sample");
    let (kick_notes, kick_params) =
        studio.add_instrument(Sampler::new(kick.data, VOICES), Some(drums.id));

    let synth = Synth::default(sample_rate);
    let synth_params = synth.params();
    let (synth_notes, synth_instrument_params) = studio.add_instrument(synth, Some(keys.id));

    let params = AppParams {
        sample_rate,
        mixer,
        transport,
        metronome,
        channels: vec![
            ChannelHandle {
                name: "Drums".to_owned(),
                params: drums,
            },
            ChannelHandle {
                name: "Keys".to_owned(),
                params: keys,
            },
        ],
        instruments: vec![
            InstrumentHandle {
                name: "Synth".to_owned(),
                kind: InstrumentKind::Synth(synth_params),
                notes: synth_notes,
                params: synth_instrument_params,
            },
            InstrumentHandle {
                name: "Kick".to_owned(),
                kind: InstrumentKind::Sampler {
                    sample: KICK_PATH.to_owned(),
                },
                notes: kick_notes,
                params: kick_params,
            },
        ],
        inputs: Vec::new(),
        commands,
        garbage,
    };

    (studio, params)
}
