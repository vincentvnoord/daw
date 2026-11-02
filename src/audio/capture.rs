use std::error::Error;

use cpal::{
    FromSample, SampleFormat,
    traits::{DeviceTrait, StreamTrait},
};
use rtrb::Producer;

/// The sample formats we can record in, best first. A device often offers
/// several; we take the first one it has.
const FORMATS: [SampleFormat; 4] = [
    SampleFormat::F32,
    SampleFormat::I32,
    SampleFormat::I16,
    SampleFormat::U16,
];

/// Starts recording `channel` (0 is the device's first input) from `device`
/// and pushes its samples into `samples`. The returned stream records for as
/// long as it's kept alive.
pub fn open_input(
    device: &cpal::Device,
    channel: usize,
    sample_rate: u32,
    samples: Producer<f32>,
) -> Result<cpal::Stream, Box<dyn Error>> {
    let configs: Vec<_> = device
        .supported_input_configs()?
        .filter(|range| range.channels() as usize > channel)
        .filter_map(|range| range.try_with_sample_rate(sample_rate))
        .collect();

    let supported = *FORMATS
        .iter()
        .find_map(|&format| configs.iter().find(|c| c.sample_format() == format))
        .ok_or("input device has no config we can record at the output's sample rate")?;

    let sample_format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();

    let stream = match sample_format {
        SampleFormat::F32 => build::<f32>(device, &config, channel, samples),
        SampleFormat::I32 => build::<i32>(device, &config, channel, samples),
        SampleFormat::I16 => build::<i16>(device, &config, channel, samples),
        SampleFormat::U16 => build::<u16>(device, &config, channel, samples),
        format => unreachable!("'{format}' isn't in FORMATS"),
    }?;

    stream.play()?;
    Ok(stream)
}

/// How many inputs the device has: the most channels any of its configs
/// records.
pub fn input_channels(device: &cpal::Device) -> usize {
    device.supported_input_configs().map_or(0, |configs| {
        configs.map(|c| c.channels() as usize).max().unwrap_or(0)
    })
}

fn build<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channel: usize,
    mut samples: Producer<f32>,
) -> Result<cpal::Stream, cpal::Error>
where
    T: cpal::SizedSample,
    f32: FromSample<T>,
{
    let channels = config.channels as usize;
    device.build_input_stream(
        *config,
        move |data: &[T], _: &cpal::InputCallbackInfo| {
            for frame in data.chunks(channels) {
                let _ = samples.push(f32::from_sample_(frame[channel]));
            }
        },
        |err| eprintln!("an error occured on the input audio stream: {}", err),
        None,
    )
}
