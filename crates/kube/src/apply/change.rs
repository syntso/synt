use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Change {
    Create,
    Update,
    Unchanged,
}

impl Change {
    pub(crate) fn between(before: Option<&str>, after: Option<&str>) -> Self {
        match before {
            None => Self::Create,
            Some(version) if after == Some(version) => Self::Unchanged,
            Some(_) => Self::Update,
        }
    }
}
