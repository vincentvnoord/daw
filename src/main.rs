use std::f32::consts::TAU;

use cpal::{
    FromSample, Sample, SampleFormat,
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
        SineOsc::new(261.63, sr),
        SineOsc::new(329.63, sr),
        SineOsc::new(392.00, sr),
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

fn build_stream<T>(
    device: cpal::Device,
    config: cpal::StreamConfig,
    mut chord: Vec<SineOsc>,
) -> Result<cpal::Stream, Box<dyn std::error::Error>>
where
    T: cpal::SizedSample + FromSample<f32>,
{
    let err_fn = |err| eprintln!("an error occured on the output audio stream: {}", err);

    let channels = config.channels as usize;
    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            write_frames(data, channels, || {
                let sum: f32 = chord.iter_mut().map(|o| o.next_sample()).sum();
                sum / chord.len() as f32 * 0.2
            });
        },
        err_fn,
        None,
    )?;

    Ok(stream)
}

fn write_frames<T: Sample + FromSample<f32>>(
    data: &mut [T],
    channels: usize,
    mut next: impl FnMut() -> f32,
) {
    for frame in data.chunks_mut(channels) {
        let value = T::from_sample_(next());
        for sample in frame.iter_mut() {
            *sample = value;
        }
    }
}

struct SineOsc {
    frequency: f32,
    sample_rate: f32,
    phase: f32,
}

impl SineOsc {
    fn new(frequency: f32, sample_rate: f32) -> SineOsc {
        SineOsc {
            frequency,
            sample_rate,
            phase: 0.0,
        }
    }

    fn next_sample(&mut self) -> f32 {
        let value = (self.phase * TAU).sin();

        self.phase += self.frequency / self.sample_rate;

        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        value
    }
}
