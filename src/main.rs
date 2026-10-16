mod note;
mod osc;
mod output;

use crate::note::Note;
use crate::{note::note_to_freq, osc::SineOsc, output::build_stream};

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

    let chord = vec![
        SineOsc::new(note_to_freq(Note::C, 4), sr),
        SineOsc::new(note_to_freq(Note::E, 4), sr),
        SineOsc::new(note_to_freq(Note::G, 4), sr),
    ];

    let stream = match sample_format {
        SampleFormat::F32 => build_stream::<f32>(device, config, chord),
        SampleFormat::I16 => build_stream::<i16>(device, config, chord),
        SampleFormat::U16 => build_stream::<u16>(device, config, chord),
        sample_format => panic!("Unsupported sample format '{sample_format}'"),
    }
    .unwrap();

    stream.play().unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));
}
