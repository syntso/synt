use crate::{ActionError, Registry};

pub trait ActionGroup {
    fn register(&self, registry: &mut Registry) -> Result<(), ActionError>;
}
