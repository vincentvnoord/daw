use cpal::{FromSample, Sample, traits::DeviceTrait};

use crate::audio::{
    engine::AudioDevice,
    frame::Frame,
    source::{MAX_BLOCK_FRAMES, StereoSource},
};

pub fn build_stream<T, A: StereoSource>(
    device: AudioDevice,
    mut source: A,
) -> Result<cpal::Stream, Box<dyn std::error::Error>>
where
    T: cpal::SizedSample + FromSample<f32>,
{
    let err_fn = |err| eprintln!("an error occured on the output audio stream: {}", err);
    let config = device.config();

    let mut block = vec![Frame::default(); MAX_BLOCK_FRAMES];

    let channels = config.channels as usize;
    let stream = device.device().build_output_stream(
        *config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            source.begin_buffer(data.len() / channels);
            for chunk in data.chunks_mut(MAX_BLOCK_FRAMES * channels) {
                let block = &mut block[..chunk.len() / channels];
                source.process(block);
                write_frames(chunk, channels, block);
            }
        },
        err_fn,
        None,
    )?;

    Ok(stream)
}

fn write_frames<T: Sample + FromSample<f32>>(chunk: &mut [T], channels: usize, frames: &[Frame]) {
    for (out, frame) in chunk.chunks_mut(channels).zip(frames) {
        if channels == 1 {
            out[0] = T::from_sample_((frame.left + frame.right) * 0.5);
        } else {
            out[0] = T::from_sample_(frame.left);
            out[1] = T::from_sample_(frame.right);

            for sample in out.iter_mut().skip(2) {
                *sample = T::EQUILIBRIUM;
            }
        }
    }
}
