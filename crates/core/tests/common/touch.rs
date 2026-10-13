use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use synt_actions::{Action, ActionError, Context, Risk};

pub struct Touch {
    touches: Arc<AtomicUsize>,
}

impl Touch {
    pub fn new(touches: Arc<AtomicUsize>) -> Self {
        Self { touches }
    }
}

impl Action for Touch {
    const ID: &'static str = "test.touch";
    const RISK: Risk = Risk::Write;

    type Params = ();
    type Output = ();

    async fn run(&self, _context: &Context, (): ()) -> Result<(), ActionError> {
        self.touches.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
