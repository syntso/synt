use synt_kube::Kind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KindFacts {
    pub(crate) secret: bool,
    pub(crate) namespaced: bool,
}

impl From<&Kind> for KindFacts {
    fn from(kind: &Kind) -> Self {
        Self {
            secret: kind.is_secret(),
            namespaced: kind.is_namespaced(),
        }
    }
}
