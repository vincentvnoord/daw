use std::error::Error;

use cpal::{
    SampleFormat,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};

use crate::audio::{latency, output::build_stream, source::StereoSource};

pub struct AudioDevice {
    device: cpal::Device,
    config: cpal::StreamConfig,
    sample_format: SampleFormat,
}

impl AudioDevice {
    /// The system's default output device, in its default config but with
    /// a small buffer, so what you play is heard quickly.
    pub fn open() -> Result<AudioDevice, Box<dyn Error>> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("no output device found")?;
        let supported_config = device.default_output_config()?;

        let sample_format = supported_config.sample_format();
        let supported_buffer = *supported_config.buffer_size();
        let mut config: cpal::StreamConfig = supported_config.into();
        config.buffer_size = latency::buffer_size(&supported_buffer);

        if latency::logging() {
            let name = device
                .description()
                .map_or_else(|_| device.to_string(), |d| d.name().to_owned());
            eprintln!(
                "[latency] output: host {}, device {name}, {} Hz, {} channels, {sample_format}, buffer {:?} (supports {supported_buffer:?})",
                host.id().name(),
                config.sample_rate,
                config.channels,
                config.buffer_size,
            );
        }

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
