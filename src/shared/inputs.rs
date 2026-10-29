use std::{error::Error, sync::Arc};

use cpal::traits::{DeviceTrait, HostTrait};

use crate::{
    audio::{
        capture::{input_channels, open_input},
        studio::{
            input::{InputParams, InputSlot},
            studio::{MAX_INPUTS, StudioCommand},
        },
    },
    shared::{AppParams, unique_name},
};

/// The UI's side of an input in the studio.
pub struct InputHandle {
    pub name: String,
    pub params: Arc<InputParams>,
    /// Keeps the device recording; dropping it stops the input.
    _stream: cpal::Stream,
}

/// One channel of a recording device, which the user can add as an input.
/// An audio interface with two inputs shows up as two of these.
pub struct InputDevice {
    /// The device's name.
    pub name: String,
    /// Which of its inputs, from 0.
    pub channel: usize,
    /// How many inputs the device has.
    pub channels: usize,
    device: cpal::Device,
}

impl InputDevice {
    /// "Input 2", or nothing for a device with a single input.
    pub fn channel_label(&self) -> Option<String> {
        (self.channels > 1).then(|| format!("Input {}", self.channel + 1))
    }

    /// What to call an input made from this: the device's name, plus which
    /// of its inputs when it has more than one.
    fn input_name(&self) -> String {
        match self.channel_label() {
            Some(label) => format!("{} · {label}", self.name),
            None => self.name.clone(),
        }
    }
}

/// Every input of every recording device on the system. Asking the system
/// can take a moment, so call this when the list is needed, not every frame.
pub fn input_devices() -> Vec<InputDevice> {
    let devices = match cpal::default_host().input_devices() {
        Ok(devices) => devices,
        Err(err) => {
            eprintln!("failed to list input devices: {err}");
            return Vec::new();
        }
    };

    devices
        .flat_map(|device| {
            let name = device
                .description()
                .map_or_else(|_| device.to_string(), |d| d.name().to_owned());
            let channels = input_channels(&device);
            (0..channels).map(move |channel| InputDevice {
                name: name.clone(),
                channel,
                channels,
                device: device.clone(),
            })
        })
        .collect()
}

impl AppParams {
    /// Starts recording from `device` and adds it as an input, playing into a
    /// new channel named after it. It starts disabled, so a mic doesn't feed
    /// back through the speakers the moment it's added. Returns its index in
    /// `inputs`, or `None` when the studio is full.
    pub fn add_input(&mut self, device: &InputDevice) -> Result<Option<usize>, Box<dyn Error>> {
        // room for both the channel and the input, so we never end up with
        // one without the other
        if self.inputs.len() >= MAX_INPUTS || self.commands.slots() < 2 {
            return Ok(None);
        }

        // opened before the channel exists, so a device that fails to open
        // doesn't leave an empty channel behind
        let (slot, samples, params) = InputSlot::create(None);
        let stream = open_input(
            &device.device,
            device.channel,
            self.sample_rate as u32,
            samples,
        )?;

        let name = unique_name(&device.input_name(), |name| {
            self.inputs.iter().any(|i| i.name == name)
        });
        // at unity: a recorded signal is quiet already, unlike a synth at full scale
        params
            .route
            .set_channel(self.add_channel_with_gain(name.clone(), 1.0));
        if self.commands.push(StudioCommand::AddInput(slot)).is_err() {
            return Ok(None);
        }

        self.inputs.push(InputHandle {
            name,
            params,
            _stream: stream,
        });
        Ok(Some(self.inputs.len() - 1))
    }
}
