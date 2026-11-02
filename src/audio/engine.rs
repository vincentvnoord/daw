use std::error::Error;

use cpal::{
    BufferSize, HostId, SampleFormat, SupportedBufferSize,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};

use crate::audio::{output::build_stream, source::StereoSource};

const ALSA_BUFFER_SIZE: u32 = 1024;

pub struct AudioDevice {
    device: cpal::Device,
    config: cpal::StreamConfig,
    sample_format: SampleFormat,
}

impl AudioDevice {
    /// The system's default output device, in its default config.
    pub fn open() -> Result<AudioDevice, Box<dyn Error>> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("no output device found")?;
        let supported_config = device.default_output_config()?;

        let sample_format = supported_config.sample_format();
        let buffer_size = match (host.id(), *supported_config.buffer_size()) {
            #[cfg(target_os = "linux")]
            (HostId::Alsa, SupportedBufferSize::Range { min, max }) => {
                BufferSize::Fixed(ALSA_BUFFER_SIZE.clamp(min, max))
            }
            _ => BufferSize::Default,
        };

        let mut config: cpal::StreamConfig = supported_config.into();
        config.buffer_size = buffer_size;

        Ok(AudioDevice {
            device,
            config,
            sample_format,
        })
    }

    pub fn sample_rate(&self) -> f32 {
        self.config.sample_rate as f32
    }

    pub fn config(&self) -> &cpal::StreamConfig {
        &self.config
    }

    pub fn device(&self) -> &cpal::Device {
        &self.device
    }
}

pub struct AudioEngine {
    /// Keeps the audio playing; dropping it stops the stream.
    _stream: cpal::Stream,
}

impl AudioEngine {
    /// Starts playing `source` on `device`.
    pub fn new<A: StereoSource>(
        device: AudioDevice,
        source: A,
    ) -> Result<AudioEngine, Box<dyn Error>> {
        let stream = match device.sample_format {
            SampleFormat::F32 => build_stream::<f32, A>(device, source),
            SampleFormat::I16 => build_stream::<i16, A>(device, source),
            SampleFormat::U16 => build_stream::<u16, A>(device, source),
            sample_format => {
                return Err(format!("unsupported sample format '{sample_format}'").into());
            }
        }?;

        stream.play()?;
        Ok(AudioEngine { _stream: stream })
    }
}
