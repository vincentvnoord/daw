use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

pub struct AtomicF32(AtomicU32);

impl AtomicF32 {
    pub fn new(value: f32) -> Self {
        Self(AtomicU32::new(value.to_bits()))
    }

    pub fn get(&self) -> f32 {
        f32::from_bits(self.0.load(Ordering::Relaxed))
    }

    pub fn set(&self, value: f32) {
        self.0.store(value.to_bits(), Ordering::Relaxed);
    }
}

pub struct AtomicF64(AtomicU64);

impl AtomicF64 {
    pub fn new(value: f64) -> Self {
        Self(AtomicU64::new(value.to_bits()))
    }

    pub fn set(&self, value: f64) {
        self.0.store(value.to_bits(), Ordering::Relaxed);
    }

    /// Stores `value` and returns what was there before, in one step, so
    /// nothing another thread stores in between can be lost.
    pub fn swap(&self, value: f64) -> f64 {
        let old_bits = self.0.swap(value.to_bits(), Ordering::Relaxed);
        f64::from_bits(old_bits)
    }
}
