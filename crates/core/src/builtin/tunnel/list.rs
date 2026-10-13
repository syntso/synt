use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_ssh::Tunnels;

use super::TunnelInfo;

#[derive(Debug, Clone)]
pub struct TunnelList {
    tunnels: Tunnels,
}

impl TunnelList {
    pub fn new(tunnels: Tunnels) -> Self {
        Self { tunnels }
    }
}

impl Action for TunnelList {
    const ID: &'static str = "tunnel.list";
    const TITLE: &'static str = "List the open tunnels";
    const RISK: Risk = Risk::Read;

    type Params = ();
    type Output = Vec<TunnelInfo>;

    fn summary(&self, output: &Vec<TunnelInfo>) -> Value {
        json!({ "tunnels": output.len() })
    }

    async fn run(&self, _context: &Context, (): ()) -> Result<Vec<TunnelInfo>, ActionError> {
        let open = self.tunnels.list();

        Ok(open.into_iter().map(TunnelInfo::from).collect())
    }
}
