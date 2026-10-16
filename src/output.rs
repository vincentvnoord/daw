use crate::osc::SineOsc;

use cpal::{FromSample, Sample, traits::DeviceTrait};

pub fn build_stream<T>(
    device: cpal::Device,
    config: cpal::StreamConfig,
    mut chord: Vec<SineOsc>,
    melody: Vec<f32>,
) -> Result<cpal::Stream, Box<dyn std::error::Error>>
where
    T: cpal::SizedSample + FromSample<f32>,
{
    let note_length = (config.sample_rate as f32 * 0.4) as u32;
    let mut counter = 0;
    let mut index = 0;
    let err_fn = |err| eprintln!("an error occured on the output audio stream: {}", err);

    let channels = config.channels as usize;
    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            write_frames(data, channels, || {
                if counter == 0 {
                    chord[0].set_frequency(melody[index]);
                }

                counter += 1;
                if counter >= note_length {
                    counter = 0;
                    index = (index + 1) % melody.len();
                }

                chord[0].next_sample() * 0.2
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
