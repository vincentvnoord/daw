use std::sync::atomic::{AtomicU32, Ordering};

pub struct Meter(AtomicU32);

impl Meter {
    pub fn new() -> Self {
        Self(AtomicU32::new(0))
    }

    pub fn record(&self, value: f32) {
        self.0.fetch_max(value.abs().to_bits(), Ordering::Relaxed);
    }

    pub fn take(&self) -> f32 {
        f32::from_bits(self.0.swap(0, Ordering::Relaxed))
    }
}
