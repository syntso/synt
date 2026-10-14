use synt_journal::ActionId;

use crate::{Description, Handle};

pub trait Catalog: Send + Sync + 'static {
    fn get(&self, id: &ActionId) -> Option<Handle>;

    fn describe(&self) -> Vec<Description>;
}
