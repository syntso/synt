use serde::Serialize;

use crate::manifest::Contributions;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Contributed {
    pub extension: String,
    pub name: String,
    pub contributes: Contributions,
}
