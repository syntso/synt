use serde::Deserialize;
use serde_json::Value;
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_ssh::{Destination, SshError, Tunnels};

use super::TunnelInfo;
use crate::builtin::ssh::{failure, hidden_root, is_root, logs_in_as_root, resolve};

#[derive(Debug, Clone)]
pub struct TunnelOpen {
    tunnels: Tunnels,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct OpenParams {
    pub remote_host: String,
    pub remote_port: u16,

    #[serde(default)]
    pub local_port: Option<u16>,
}

impl TunnelOpen {
    pub fn new(tunnels: Tunnels) -> Self {
        Self { tunnels }
    }
}

impl Action for TunnelOpen {
    const ID: &'static str = "tunnel.open";
    const RISK: Risk = Risk::Write;

    type Params = OpenParams;
    type Output = TunnelInfo;

    fn risk(&self, target: Option<&ResourceRef>, _params: &OpenParams) -> Risk {
        if logs_in_as_root(target) {
            return Risk::Privileged;
        }

        Risk::Write
    }

    fn summary(&self, output: &TunnelInfo) -> Value {
        output.ports()
    }

    async fn run(&self, context: &Context, params: OpenParams) -> Result<TunnelInfo, ActionError> {
        let host = context.target().ok_or(ActionError::TargetRequired)?;
        let destination = destination(&params)?;
        let local_port = local_port(params.local_port)?;

        let target = resolve(host.as_str()).await?;

        if is_root(target.user()) && context.risk() != Risk::Privileged {
            return Err(hidden_root(host));
        }

        let opened = self
            .tunnels
            .open(&target, destination, local_port)
            .await
            .map_err(|error| open_failure(&error))?;

        Ok(TunnelInfo::from(opened))
    }
}

fn destination(params: &OpenParams) -> Result<Destination, ActionError> {
    let host = params.remote_host.trim();

    if host.is_empty() || host.contains(|character: char| character.is_whitespace()) {
        return Err(ActionError::InvalidParams(format!(
            "remote_host must be a host name or address, got {:?}",
            params.remote_host
        )));
    }

    if params.remote_port == 0 {
        return Err(ActionError::InvalidParams(
            "remote_port must be between 1 and 65535".to_owned(),
        ));
    }

    Ok(Destination::new(host, params.remote_port))
}

fn local_port(requested: Option<u16>) -> Result<Option<u16>, ActionError> {
    if requested == Some(0) {
        return Err(ActionError::InvalidParams(
            "local_port must be between 1 and 65535; leave it out to pick a free port".to_owned(),
        ));
    }

    Ok(requested)
}

fn open_failure(error: &SshError) -> ActionError {
    match error {
        SshError::Timeout(limit) => ActionError::Failed(format!(
            "could not connect within {} s, so no tunnel was opened",
            limit.as_secs()
        )),
        SshError::Listen { .. } => ActionError::Failed(error.to_string()),
        other => failure(other),
    }
}
