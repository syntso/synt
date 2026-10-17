use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use synt_actions::{Action, ActionError, Context, Risk};
use tokio::time;

const LONGEST_WAIT: Duration = Duration::from_secs(30);

pub struct Waiting {
    started: Arc<AtomicUsize>,
}

impl Waiting {
    pub const STOPPED: &'static str = "stopped by the core";

    pub fn new(started: Arc<AtomicUsize>) -> Self {
        Self { started }
    }
}

impl Action for Waiting {
    const ID: &'static str = "test.wait";
    const TITLE: &'static str = "Wait until the core stops the call";
    const RISK: Risk = Risk::Write;

    type Params = ();
    type Output = ();

    async fn run(&self, context: &Context, (): ()) -> Result<(), ActionError> {
        self.started.fetch_add(1, Ordering::SeqCst);

        tokio::select! {
            () = context.stop().cancelled() => Err(ActionError::failed(Self::STOPPED)),
            () = time::sleep(LONGEST_WAIT) => Ok(()),
        }
    }
}
