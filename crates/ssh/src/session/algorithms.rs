use russh::Names;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Algorithms {
    pub kex: String,
    pub cipher: String,
}

impl Algorithms {
    pub(crate) fn negotiated(names: &Names) -> Self {
        Self {
            kex: names.kex.as_ref().to_owned(),
            cipher: names.cipher.as_ref().to_owned(),
        }
    }
}
