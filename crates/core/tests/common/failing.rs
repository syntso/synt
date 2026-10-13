use synt_actions::{Action, ActionError, Context, Risk};

pub struct Failing;

impl Failing {
    pub const REASON: &'static str = "the disk is full";
}

impl Action for Failing {
    const ID: &'static str = "test.fail";
    const RISK: Risk = Risk::Read;

    type Params = ();
    type Output = ();

    async fn run(&self, _context: &Context, (): ()) -> Result<(), ActionError> {
        Err(ActionError::failed(Self::REASON))
    }
}
