use std::{collections::BTreeSet, default};

use eframe::egui::{self, Event};

use crate::{events::NoteEvent, ui::keyboard::key_to_note_event};

pub struct App {
    note_producer: rtrb::Producer<NoteEvent>,
    held_keys: BTreeSet<u8>,
}

impl App {
    pub fn new(note_producer: rtrb::Producer<NoteEvent>) -> Self {
        Self {
            note_producer,
            held_keys: BTreeSet::new(),
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.input(|i| {
            for ev in &i.events {
                if let Some(note_event) = key_to_note_event(ev) {
                    match note_event {
                        NoteEvent::On(n) => {
                            self.held_keys.insert(n);
                        }
                        NoteEvent::Off(n) => {
                            self.held_keys.remove(&n);
                        }
                    }

                    let _ = self.note_producer.push(note_event);
                }
            }
        });

        ui.heading("Pressed keys");
        ui.label(format!("held notes: {:?}", self.held_keys));
    }
}

