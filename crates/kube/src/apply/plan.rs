mod object;

pub use object::PlannedObject;

use serde::Serialize;

use crate::Change;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Plan {
    pub objects: Vec<PlannedObject>,
}

impl Plan {
    pub fn diff(&self) -> String {
        self.objects
            .iter()
            .map(|planned| planned.diff.as_str())
            .filter(|diff| !diff.is_empty())
            .collect()
    }

    pub fn count(&self, change: Change) -> usize {
        self.objects
            .iter()
            .filter(|planned| planned.change == change)
            .count()
    }
}
