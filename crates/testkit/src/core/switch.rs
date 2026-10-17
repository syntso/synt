use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use synt_core::Activity;

#[derive(Clone, Default)]
pub struct Switch {
    on: Arc<AtomicBool>,
}

impl Switch {
    pub fn set(&self, on: bool) {
        self.on.store(on, Ordering::SeqCst);
    }
}

impl Activity for Switch {
    fn is_active(&self) -> bool {
        self.on.load(Ordering::SeqCst)
    }
}
