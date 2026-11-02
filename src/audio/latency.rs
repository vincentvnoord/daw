//! Keeping the round trip from input to output short, and the logging to
//! see where it goes. Run with `DAW_LOG_LATENCY=1` to print the stream
//! configs as they open and the buffer stats once a second.

use std::sync::OnceLock;

/// Frames per buffer asked of the output and input streams: about 5 ms at
/// 48 kHz. The device may give a different size; the log shows what it did.
pub const BUFFER_FRAMES: u32 = 256;

/// Whether `DAW_LOG_LATENCY` is set.
pub fn logging() -> bool {
    static LOGGING: OnceLock<bool> = OnceLock::new();
    *LOGGING.get_or_init(|| std::env::var_os("DAW_LOG_LATENCY").is_some())
}

/// What `cpal` should ask the device for: `BUFFER_FRAMES` when the device says
/// what it supports, clamped to that; the device's own choice otherwise.
pub fn buffer_size(supported: &cpal::SupportedBufferSize) -> cpal::BufferSize {
    match *supported {
        cpal::SupportedBufferSize::Range { min, max } => {
            cpal::BufferSize::Fixed(BUFFER_FRAMES.clamp(min, max))
        }
        cpal::SupportedBufferSize::Unknown => cpal::BufferSize::Default,
    }
}

/// `frames` at `sample_rate` in milliseconds, for the log.
pub fn millis(frames: u32, sample_rate: u32) -> f32 {
    frames as f32 * 1000.0 / sample_rate as f32
}
