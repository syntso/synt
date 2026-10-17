use synt_actions::{ActionError, Context};
use synt_journal::{Actor, Name};

pub(crate) trait CallingActor {
    fn refuse_extensions(&self, reaching: &str) -> Result<(), ActionError>;

    fn calling_extension(&self) -> Option<&Name>;
}

impl CallingActor for Context {
    fn refuse_extensions(&self, reaching: &str) -> Result<(), ActionError> {
        if self.calling_extension().is_some() {
            return Err(ActionError::failed(format!(
                "an extension cannot {reaching}"
            )));
        }

        Ok(())
    }

    fn calling_extension(&self) -> Option<&Name> {
        match self.actor() {
            Actor::Plugin(name) => Some(name),
            _ => None,
        }
    }
}
