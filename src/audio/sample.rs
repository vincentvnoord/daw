use std::{error::Error, path::Path, sync::Arc};

use hound::WavReader;
use rubato::{Fft, FixedSync, Resampler, audioadapter_buffers::direct::InterleavedSlice};

#[derive(Debug)]
pub struct Sample {
    pub data: Arc<Vec<f32>>,
    pub sample_rate: f64,
}

impl Sample {
    pub fn load(path: &Path, sample_rate: f32) -> Result<Sample, Box<dyn Error>> {
        let mut reader = WavReader::open(path)?;
        let spec = reader.spec();
        let interleaved: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Float => reader.samples::<f32>().collect::<Result<_, _>>()?,
            hound::SampleFormat::Int => {
                let max = (1i64 << (spec.bits_per_sample - 1)) as f32;
                reader
                    .samples::<i32>()
                    .map(|s| s.map(|v| v as f32 / max))
                    .collect::<Result<_, _>>()?
            }
        };

        let channels = spec.channels as usize;
        let mono: Vec<f32> = interleaved
            .chunks(channels)
            .map(|frame| frame.iter().sum::<f32>() / channels as f32)
            .collect();

        let mut sample = Sample {
            data: Arc::new(mono),
            sample_rate: spec.sample_rate as f64,
        };

        sample.resample_to(sample_rate);

        Ok(sample)
    }

    pub fn resample_to(&mut self, sample_rate: f32) -> Result<(), Box<dyn Error>> {
        let from = self.sample_rate as usize;
        let to = sample_rate as usize;
        if from == to {
            return Ok(());
        }

        let frames = self.data.len();
        let input = InterleavedSlice::new(&self.data[..], 1, frames)?;

        let mut resampler = Fft::<f32>::new(from, to, 1024, 1, FixedSync::Both)?;
        let output = resampler.process_all(&input, frames, None)?;

        self.data = Arc::new(output.take_data());
        self.sample_rate = sample_rate as f64;

        Ok(())
    }
}
