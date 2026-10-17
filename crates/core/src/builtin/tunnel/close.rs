use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_ssh::{TunnelId, Tunnels};

use super::TunnelInfo;

#[derive(Debug, Clone)]
pub struct TunnelClose {
    tunnels: Tunnels,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
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
    const TITLE: &'static str = "Close a tunnel";
    const RISK: Risk = Risk::Write;

    type Params = CloseParams;
    type Output = TunnelInfo;

    fn risk(&self, _target: Option<&ResourceRef>, params: &CloseParams) -> Risk {
        if self.tunnels.logs_in_as_root(TunnelId::from(params.id)) {
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
