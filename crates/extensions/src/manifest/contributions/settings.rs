mod schema;

use serde::{Deserialize, Serialize};

pub use schema::SettingsSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    schema: SettingsSchema,
}

impl Settings {
    pub(crate) fn schema(&self) -> &SettingsSchema {
        &self.schema
    }
}
