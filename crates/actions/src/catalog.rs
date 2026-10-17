use synt_journal::ActionId;

use crate::{ActionDescription, RegisteredAction};

pub trait Catalog: Send + Sync + 'static {
    fn get(&self, id: &ActionId) -> Option<RegisteredAction>;

    fn describe(&self) -> Vec<ActionDescription>;
}
