use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_ssh::{TunnelId, Tunnels};

use super::TunnelInfo;
use crate::builtin::ssh::is_root;

#[derive(Debug, Clone)]
pub struct TunnelClose {
    tunnels: Tunnels,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CloseParams {
    pub id: u64,
}

impl TunnelClose {
    pub fn new(tunnels: Tunnels) -> Self {
        Self { tunnels }
    }
}

impl Action for TunnelClose {
    const ID: &'static str = "tunnel.close";
    const RISK: Risk = Risk::Write;

    type Params = CloseParams;
    type Output = TunnelInfo;

    fn risk(&self, _target: Option<&ResourceRef>, params: &CloseParams) -> Risk {
        let user = self.tunnels.login_user(TunnelId::from(params.id));

        if user.as_deref().is_some_and(is_root) {
            return Risk::Privileged;
        }

        Risk::Write
    }

    fn summary(&self, output: &TunnelInfo) -> Value {
        let mut summary = output.ports();

        if let Value::Object(fields) = &mut summary {
            fields.insert("connections".to_owned(), json!(output.connections));
            fields.insert("sent_bytes".to_owned(), json!(output.sent_bytes));
            fields.insert("received_bytes".to_owned(), json!(output.received_bytes));
        }

        summary
    }

    async fn run(
        &self,
        _context: &Context,
        params: CloseParams,
    ) -> Result<TunnelInfo, ActionError> {
        let id = TunnelId::from(params.id);

        let closed = self
            .tunnels
            .close(id)
            .await
            .ok_or_else(|| ActionError::Failed(format!("there is no open tunnel {id}")))?;

        Ok(TunnelInfo::from(closed))
    }
}
