use cpal::{FromSample, Sample, traits::DeviceTrait};

use crate::audio::{engine::AudioDevice, frame::Frame, source::StereoSource};

pub fn build_stream<T, A: StereoSource>(
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
            write_frames(data, channels, || source.next_frame());
        },
        err_fn,
        None,
    )?;

    Ok(stream)
}

fn write_frames<T: Sample + FromSample<f32>>(
    data: &mut [T],
    channels: usize,
    mut next: impl FnMut() -> Frame,
) {
    for chunk in data.chunks_mut(channels) {
        let frame = next();
        if channels == 1 {
            chunk[0] = T::from_sample_((frame.left + frame.right) * 0.5);
        } else {
            chunk[0] = T::from_sample_(frame.left);
            chunk[1] = T::from_sample_(frame.right);

            for sample in chunk.iter_mut().skip(2) {
                *sample = T::EQUILIBRIUM;
            }
        }
    }
}
