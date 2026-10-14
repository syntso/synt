#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Merged {
    pub(crate) body: Option<Vec<u8>>,
    pub(crate) conflicts: Vec<String>,
}

impl Merged {
    pub(crate) fn clean(body: Option<&[u8]>) -> Self {
        Self {
            body: body.map(<[u8]>::to_vec),
            conflicts: Vec::new(),
        }
    }

    pub(crate) fn conflicted(body: Option<&[u8]>, conflict: &str) -> Self {
        Self {
            body: body.map(<[u8]>::to_vec),
            conflicts: vec![conflict.to_owned()],
        }
    }
}
