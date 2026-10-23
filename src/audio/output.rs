use cpal::{FromSample, Sample, traits::DeviceTrait};

use crate::audio::{engine::AudioDevice, source::AudioSource};

pub fn build_stream<T, A: AudioSource>(
    device: AudioDevice,
    mut source: A,
) -> Result<cpal::Stream, Box<dyn std::error::Error>>
where
    T: cpal::SizedSample + FromSample<f32>,
{
    let err_fn = |err| eprintln!("an error occured on the output audio stream: {}", err);
    let config = device.config();

    let channels = config.channels as usize;
    let stream = device.device().build_output_stream(
        *config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            source.begin_buffer();
            write_frames(data, channels, || source.next_sample());
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
