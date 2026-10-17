use std::sync::{Mutex, PoisonError};

use synt_extensions::{Screen, ScreenUpdate};

#[derive(Default)]
pub struct RecordingScreen {
    updates: Mutex<Vec<ScreenUpdate>>,
}

impl RecordingScreen {
    pub fn updates(&self) -> Vec<ScreenUpdate> {
        self.updates
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Screen for RecordingScreen {
    fn show(&self, update: ScreenUpdate) {
        self.updates
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(update);
    }
}
