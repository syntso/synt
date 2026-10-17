use synt_actions::{Action, ActionError, Context, Risk};

pub struct Panicking;

impl Action for Panicking {
    const ID: &'static str = "test.panic";
    const TITLE: &'static str = "Always panic";
    const RISK: Risk = Risk::Read;

    type Params = ();
    type Output = ();

    async fn run(&self, _context: &Context, (): ()) -> Result<(), ActionError> {
        panic!("the action lost its footing");
    }
}
