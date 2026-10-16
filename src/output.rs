use crate::osc::SineOsc;

use cpal::{
    FromSample, Sample,
    traits::{DeviceTrait},
};

pub fn build_stream<T>(
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
