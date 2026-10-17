use synt_actions::{Action, ActionError, Context, Risk};

pub struct Confused;

impl Action for Confused {
    const ID: &'static str = "test.confused";
    const TITLE: &'static str = "Fail the way only a bug in the core can";
    const RISK: Risk = Risk::Read;

    type Params = ();
    type Output = ();

    async fn run(&self, _context: &Context, (): ()) -> Result<(), ActionError> {
        Err(ActionError::Duplicate(Self::ID))
    }
}
