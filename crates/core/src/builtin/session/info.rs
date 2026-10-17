use serde::Serialize;
use synt_actions::Risk;
use synt_proto::Requester;

use crate::broker::requester;
use crate::clock::milliseconds;
use crate::terminals::{Session, SessionId, SessionKind};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SessionInfo {
    pub session: SessionId,
    pub kind: SessionKind,
    pub target: Option<String>,
    pub host: String,
    pub actor: Requester,
    pub risk: Risk,
    #[serde(rename = "cols")]
    pub columns: u16,
    pub rows: u16,
    pub attached: usize,
    pub blocks: usize,
    pub age_ms: u64,
}

impl SessionInfo {
    pub(super) async fn of(session: &Session) -> Self {
        let origin = session.origin();
        let size = session.size().await;

        Self {
            session: session.id(),
            kind: session.kind(),
            target: origin
                .target
                .as_ref()
                .map(|target| target.as_str().to_owned()),
            host: session.host(),
            actor: requester(&origin.actor),
            risk: origin.risk,
            columns: size.columns,
            rows: size.rows,
            attached: session.attached().await,
            blocks: session.blocks().await.len(),
            age_ms: milliseconds(session.age()),
        }
    }
}
