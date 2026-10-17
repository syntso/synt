use serde::Deserialize;

use super::Field;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub(in crate::view) enum FormChild {
    Field {
        #[serde(flatten)]
        _field: Field,
    },
}
