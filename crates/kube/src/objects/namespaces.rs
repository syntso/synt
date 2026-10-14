#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Namespaces {
    #[default]
    Default,
    Only(String),
    All,
}

impl Namespaces {
    pub fn from_request(namespace: Option<String>, all: bool) -> Self {
        match (namespace, all) {
            (_, true) => Self::All,
            (Some(namespace), false) => Self::Only(namespace),
            (None, false) => Self::Default,
        }
    }
}
