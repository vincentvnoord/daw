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
