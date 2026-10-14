#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IndexPart {
    pub(crate) index: String,
    pub(crate) column: String,
    pub(crate) unique: bool,
    pub(crate) primary: bool,
}
