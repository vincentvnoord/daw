use std::ops::RangeInclusive;

/// How a parameter value is displayed.
#[derive(Clone, Copy)]
pub enum Unit {
    Hz,
    Seconds,
    Percent,
    Decibels,
    /// -1 is hard left, +1 hard right.
    Pan,
}

impl Unit {
    pub fn format(self, v: f32) -> String {
        match self {
            Unit::Hz if v >= 1000.0 => format!("{:.2} kHz", v / 1000.0),
            Unit::Hz => format!("{v:.0} Hz"),
            Unit::Seconds if v < 1.0 => format!("{:.0} ms", v * 1000.0),
            Unit::Seconds => format!("{v:.2} s"),
            Unit::Percent => format!("{:.0}%", v * 100.0),
            Unit::Decibels if v <= 0.0 => "-inf dB".to_owned(),
            Unit::Decibels => format!("{:.1} dB", 20.0 * v.log10()),
            Unit::Pan => match (v * 100.0).round() as i32 {
                0 => "C".to_owned(),
                p if p < 0 => format!("L{}", -p),
                p => format!("R{p}"),
            },
        }
    }
}

/// Maps between a parameter range and a normalized 0..=1 position.
#[derive(Clone)]
pub struct Mapping {
    min: f32,
    max: f32,
    log: bool,
}

impl Mapping {
    pub fn new(range: RangeInclusive<f32>, log: bool) -> Self {
        let (mut min, max) = (*range.start(), *range.end());
        if log {
            // log scale can't start at zero
            min = min.max(1e-3);
        }
        Self { min, max, log }
    }

    pub fn to_normalized(&self, v: f32) -> f32 {
        let v = v.clamp(self.min, self.max);
        let t = if self.log {
            (v / self.min).ln() / (self.max / self.min).ln()
        } else {
            (v - self.min) / (self.max - self.min)
        };
        t.clamp(0.0, 1.0)
    }

    pub fn from_normalized(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        if self.log {
            self.min * (self.max / self.min).powf(t)
        } else {
            self.min + t * (self.max - self.min)
        }
    }
}
