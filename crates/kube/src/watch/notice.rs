mod kind;

pub use kind::NoticeKind;

use serde::Serialize;
use serde_json::Value;

use crate::objects::Fields;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WatchNotice {
    #[serde(rename = "type")]
    pub kind: NoticeKind,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_version: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<Value>,
}

impl WatchNotice {
    pub(crate) fn about(kind: NoticeKind, object: Value) -> Self {
        let field = |name: &str| object.metadata_text(name).map(str::to_owned);

        Self {
            kind,
            name: field("name"),
            namespace: field("namespace"),
            resource_version: field("resourceVersion"),
            object: Some(object),
        }
    }

    pub(crate) fn synced(resource_version: Option<String>) -> Self {
        Self {
            kind: NoticeKind::Synced,
            name: None,
            namespace: None,
            resource_version,
            object: None,
        }
    }
}
