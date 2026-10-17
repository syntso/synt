use synt_journal::ActionId;

use crate::{ActionDescription, Handle};

pub trait Catalog: Send + Sync + 'static {
    fn get(&self, id: &ActionId) -> Option<Handle>;

    fn describe(&self) -> Vec<ActionDescription>;
}
