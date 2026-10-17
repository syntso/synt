use synt_actions::{Action, ActionError, Context, Risk};

pub struct Secret;

impl Action for Secret {
    const ID: &'static str = "test.secret";
    const TITLE: &'static str = "Show something only the person may see";
    const RISK: Risk = Risk::Read;
    const PERSON_ONLY: bool = true;

    type Params = ();
    type Output = ();

    async fn run(&self, _context: &Context, (): ()) -> Result<(), ActionError> {
        Ok(())
    }
}
