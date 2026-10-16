use cpal::{
    Sample, SampleFormat,
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

    let err_fn = |err| eprintln!("an error occured on the output audio stream: {}", err);

    let sample_format = supported_config.sample_format();
    let config: cpal::StreamConfig = supported_config.into();

    let stream = match sample_format {
        SampleFormat::F32 => device.build_output_stream(config, write_silence::<f32>, err_fn, None),
        SampleFormat::I16 => device.build_output_stream(config, write_silence::<i16>, err_fn, None),
        SampleFormat::U16 => device.build_output_stream(config, write_silence::<u16>, err_fn, None),
        sample_format => panic!("Unsupported sample format '{sample_format}'"),
    }
    .unwrap();

    stream.play().unwrap();
}

fn write_silence<T: Sample>(data: &mut [T], _: &cpal::OutputCallbackInfo) {
    for sample in data.iter_mut() {
        *sample = Sample::EQUILIBRIUM;
    }
}
