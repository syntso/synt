use serde::{Deserialize, Serialize};

use super::{DocumentPath, Draft};
use crate::SyncError;
use crate::encoding::Base64;
use crate::id::MemberId;
use crate::provider::SealedRevision;

pub(crate) const LARGEST_BODY: usize = 1024 * 1024;
const LONGEST_CONFLICT_LIST: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Payload {
    pub(crate) path: String,
    pub(crate) kind: String,
    pub(crate) body: Option<Base64>,
    pub(crate) conflicts: Vec<String>,
    pub(crate) author: Option<MemberId>,
}

impl Payload {
    pub(crate) fn into_draft(self, sealed: &SealedRevision) -> Result<Draft, SyncError> {
        let path = DocumentPath::parse(&self.path)?;

        if path.kind().to_string() != self.kind {
            return Err(SyncError::refused(format!(
                "{path} is not a document of kind {}",
                self.kind
            )));
        }

        let body = self.body.map(Base64::into_bytes);
        if body.as_ref().is_some_and(|body| body.len() > LARGEST_BODY) {
            return Err(SyncError::refused(format!(
                "{path} is larger than {LARGEST_BODY} bytes"
            )));
        }

        if self.conflicts.len() > LONGEST_CONFLICT_LIST {
            return Err(SyncError::refused(format!(
                "{path} lists too many conflicts"
            )));
        }

        Ok(Draft {
            path,
            parents: sealed.parents.clone(),
            at: sealed.at,
            body,
            conflicts: self.conflicts,
            author: self.author,
        })
    }
}
