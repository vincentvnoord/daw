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
    pub fn default() -> AudioDevice {
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
        let buffer_size = match (host.id(), *supported_config.buffer_size()) {
            #[cfg(target_os = "linux")]
            (HostId::Alsa, SupportedBufferSize::Range { min, max }) => {
                BufferSize::Fixed(ALSA_BUFFER_SIZE.clamp(min, max))
            }
            _ => BufferSize::Default,
        };

        let mut config: cpal::StreamConfig = supported_config.into();
        config.buffer_size = buffer_size;

        AudioDevice {
            device,
            config,
            sample_format,
        }
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
    stream: cpal::Stream,
}

impl AudioEngine {
    pub fn new<A: StereoSource>(device: AudioDevice, mixer: A) -> AudioEngine {
        let stream = match device.sample_format {
            SampleFormat::F32 => build_stream::<f32, A>(device, mixer),
            SampleFormat::I16 => build_stream::<i16, A>(device, mixer),
            SampleFormat::U16 => build_stream::<u16, A>(device, mixer),
            sample_format => panic!("Unsupported sample format '{sample_format}'"),
        }
        .unwrap();

        stream.play().unwrap();
        AudioEngine { stream }
    }
}
