#[derive(Clone, Copy)]
pub enum State {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

#[derive(Clone, Copy)]
pub struct EnvelopeSettings {
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

pub struct Envelope {
    settings: EnvelopeSettings,
    value: f32,
    state: State,
    sample_rate: f32,
}

impl Envelope {
    pub fn new(settings: EnvelopeSettings, sample_rate: f32) -> Envelope {
        Envelope {
            settings,
            sample_rate,
            value: 0.0,
            state: State::Idle,
        }
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn trigger(&mut self) {
        self.state = State::Attack;
    }

    pub fn release(&mut self) {
        self.state = State::Release;
    }

    pub fn is_idle(&self) -> bool {
        matches!(self.state, State::Idle)
    }

    pub fn set_settings(&mut self, settings: EnvelopeSettings) {
        self.settings = settings;
    }

    pub fn next_value(&mut self) -> f32 {
        match self.state {
            State::Idle => {
                self.value = 0.0;
            }
            State::Attack => {
                self.value += 1.0 / (self.settings.attack * self.sample_rate);
                if self.value >= 1.0 {
                    self.value = 1.0;
                    self.state = State::Decay;
                }
            }
            State::Decay => {
                self.value -=
                    (1.0 - self.settings.sustain) / (self.settings.decay * self.sample_rate);
                if self.value <= self.settings.sustain {
                    self.value = self.settings.sustain;
                    self.state = State::Sustain;
                }
            }
            State::Sustain => {
                self.value = self.settings.sustain;
            }
            State::Release => {
                self.value -= 1.0 / (self.settings.release * self.sample_rate);
                if self.value <= 0.0 {
                    self.value = 0.0;
                    self.state = State::Idle;
                }
            }
        }

        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // at 4 samples per second, a quarter-second step is one sample, so the
    // ramps land on exact values
    const SAMPLE_RATE: f32 = 4.0;

    fn envelope() -> Envelope {
        let settings = EnvelopeSettings {
            attack: 1.0,
            decay: 1.0,
            sustain: 0.5,
            release: 1.0,
        };
        Envelope::new(settings, SAMPLE_RATE)
    }

    fn run(envelope: &mut Envelope, samples: usize) -> f32 {
        (0..samples).map(|_| envelope.next_value()).last().unwrap()
    }

    #[test]
    fn silent_until_triggered() {
        let mut envelope = envelope();

        assert_eq!(run(&mut envelope, 10), 0.0);
        assert!(envelope.is_idle());
    }

    #[test]
    fn attack_ramps_up_to_full() {
        let mut envelope = envelope();
        envelope.trigger();

        assert_eq!(run(&mut envelope, 2), 0.5);
        assert_eq!(run(&mut envelope, 2), 1.0);
        assert!(matches!(envelope.state(), State::Decay));
    }

    #[test]
    fn decay_settles_on_sustain() {
        let mut envelope = envelope();
        envelope.trigger();
        run(&mut envelope, 4);

        assert_eq!(run(&mut envelope, 4), 0.5);
        assert!(matches!(envelope.state(), State::Sustain));
        assert_eq!(run(&mut envelope, 100), 0.5);
    }

    #[test]
    fn release_fades_to_idle() {
        let mut envelope = envelope();
        envelope.trigger();
        run(&mut envelope, 8);
        envelope.release();

        assert_eq!(run(&mut envelope, 2), 0.0);
        assert!(envelope.is_idle());
    }
}
