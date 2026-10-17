use std::sync::{Arc, Mutex, PoisonError};

use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

pub struct Aimed {
    aim: Arc<Mutex<String>>,
}

impl Aimed {
    pub fn new(aim: Arc<Mutex<String>>) -> Self {
        Self { aim }
    }
}

impl Action for Aimed {
    const ID: &'static str = "test.aim";
    const TITLE: &'static str = "Change a place that can move between calls";
    const RISK: Risk = Risk::Write;
    const TAKES_TARGET: bool = true;

    type Params = ();
    type Output = ();

    fn destination(&self, target: Option<&ResourceRef>, (): &()) -> Option<String> {
        let aim = self
            .aim
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();

        target.map(|target| format!("{target} = {aim}"))
    }

    async fn run(&self, _context: &Context, (): ()) -> Result<(), ActionError> {
        Ok(())
    }
}
