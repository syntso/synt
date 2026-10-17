mod document;
mod graph;
mod three_way;
mod toml;

pub use document::DocumentMerger;
pub(crate) use graph::DocumentGraph;
pub(crate) use three_way::ThreeWay;
pub(crate) use toml::TomlText;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Merged {
    pub body: Option<Vec<u8>>,
    pub conflicts: Vec<String>,
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
