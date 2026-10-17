mod envelope;
mod note;
mod osc;
mod output;
mod sequence;
mod voice;

use std::{thread::sleep, time::Duration};

use crate::{
    envelope::EnvelopeSettings,
    osc::{Oscillator, WaveForm},
    output::build_stream,
    sequence::{NoteEvent, SequencePlayer},
    voice::Voice,
};

use cpal::{
    SampleFormat,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};

fn main() {
    let host = cpal::default_host();
    println!("{:?}", host.id().name());

    let device = host
        .default_output_device()
        .expect("no output device found");
    println!("{:?}", device);

    let supported_config = device
        .default_output_config()
        .expect("no default output config");

    println!("{:?}", supported_config.sample_format());

    let sample_format = supported_config.sample_format();
    let config: cpal::StreamConfig = supported_config.into();
    let sr = config.sample_rate as f32;

    let env_settings = EnvelopeSettings {
        attack: 0.005,
        decay: 0.5,
        sustain: 1.0,
        release: 0.8,
    };

    let melody = vec![
        // bar 1
        NoteEvent {
            note: 69,
            start: 0.0,
            length: 0.4,
        },
        NoteEvent {
            note: 72,
            start: 0.5,
            length: 0.2,
        },
        NoteEvent {
            note: 76,
            start: 0.75,
            length: 0.2,
        },
        NoteEvent {
            note: 74,
            start: 1.0,
            length: 0.4,
        },
        NoteEvent {
            note: 72,
            start: 1.5,
            length: 0.2,
        },
        NoteEvent {
            note: 69,
            start: 1.75,
            length: 0.2,
        },
        // bar 2
        NoteEvent {
            note: 67,
            start: 2.0,
            length: 0.4,
        },
        NoteEvent {
            note: 69,
            start: 2.5,
            length: 0.2,
        },
        NoteEvent {
            note: 72,
            start: 2.75,
            length: 0.2,
        },
        NoteEvent {
            note: 74,
            start: 3.0,
            length: 0.9,
        },
        // bar 3
        NoteEvent {
            note: 69,
            start: 4.0,
            length: 0.4,
        },
        NoteEvent {
            note: 72,
            start: 4.5,
            length: 0.2,
        },
        NoteEvent {
            note: 76,
            start: 4.75,
            length: 0.2,
        },
        NoteEvent {
            note: 79,
            start: 5.0,
            length: 0.4,
        },
        NoteEvent {
            note: 76,
            start: 5.5,
            length: 0.2,
        },
        NoteEvent {
            note: 74,
            start: 5.75,
            length: 0.2,
        },
        // bar 4
        NoteEvent {
            note: 72,
            start: 6.0,
            length: 0.4,
        },
        NoteEvent {
            note: 74,
            start: 6.5,
            length: 0.2,
        },
        NoteEvent {
            note: 76,
            start: 6.75,
            length: 0.2,
        },
        NoteEvent {
            note: 69,
            start: 7.0,
            length: 0.9,
        },
        // bar 5
        NoteEvent {
            note: 76,
            start: 8.0,
            length: 0.2,
        },
        NoteEvent {
            note: 74,
            start: 8.25,
            length: 0.2,
        },
        NoteEvent {
            note: 72,
            start: 8.5,
            length: 0.2,
        },
        NoteEvent {
            note: 74,
            start: 8.75,
            length: 0.2,
        },
        NoteEvent {
            note: 69,
            start: 9.0,
            length: 0.8,
        },
    ];

    let osc = Oscillator::new(WaveForm::Square, 440.0, sr);
    let env = envelope::Envelope::new(env_settings, sr);
    let voice = Voice::new(osc, env);
    let player = SequencePlayer::new(melody, 11.0, sr);

    let stream = match sample_format {
        SampleFormat::F32 => build_stream::<f32>(device, config, player, voice),
        SampleFormat::I16 => build_stream::<i16>(device, config, player, voice),
        SampleFormat::U16 => build_stream::<u16>(device, config, player, voice),
        sample_format => panic!("Unsupported sample format '{sample_format}'"),
    }
    .unwrap();

    stream.play().unwrap();

    sleep(Duration::from_secs(10));
}
