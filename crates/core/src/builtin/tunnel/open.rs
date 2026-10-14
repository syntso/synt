use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_ssh::{Destination, SshError, Tunnels};

use super::TunnelInfo;
use crate::builtin::hold::keep_hold_while_open;
use crate::builtin::ssh::failure;
use crate::projects::{Projects, Resolved};

#[derive(Debug, Clone)]
pub struct TunnelOpen {
    tunnels: Tunnels,
    projects: Projects,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct OpenParams {
    #[serde(default)]
    pub remote_host: Option<String>,

    #[serde(default)]
    pub remote_port: Option<u16>,

    #[serde(default)]
    pub local_port: Option<u16>,
}

impl TunnelOpen {
    pub fn new(tunnels: Tunnels, projects: Projects) -> Self {
        Self { tunnels, projects }
    }
}

impl Action for TunnelOpen {
    const ID: &'static str = "tunnel.open";
    const TITLE: &'static str = "Forward a local port to an address seen from a host";
    const RISK: Risk = Risk::Write;
    const TAKES_TARGET: bool = true;

    type Params = OpenParams;
    type Output = TunnelInfo;

    fn check(&self, target: Option<&ResourceRef>, params: &OpenParams) -> Result<(), ActionError> {
        let Some(resolved) = target.and_then(|target| self.projects.resolve(target.as_str()).ok())
        else {
            return Ok(());
        };

        plan(resolved, params).map(|_| ())
    }

    fn risk(&self, target: Option<&ResourceRef>, _params: &OpenParams) -> Risk {
        self.projects.login_risk(target, Self::RISK)
    }

    fn destination(&self, target: Option<&ResourceRef>, params: &OpenParams) -> Option<String> {
        let target = target?;
        let resolved = self.projects.resolve(target.as_str()).ok()?;
        let planned = plan(resolved, params).ok()?;

        Some(self.projects.destination(target.as_str(), &planned))
    }

    fn summary(&self, output: &TunnelInfo) -> Value {
        output.ports()
    }

    async fn run(&self, context: &Context, params: OpenParams) -> Result<TunnelInfo, ActionError> {
        let host = context.target().ok_or(ActionError::TargetRequired)?;

        let resolved = self.projects.resolve_any(host.as_str()).await?;
        let planned = plan(resolved, &params)?;
        self.projects.check_reach(context, &planned)?;

        let Resolved::Tunnel {
            host: target,
            remote,
            local,
        } = planned
        else {
            return Err(ActionError::failed("the tunnel could not be planned"));
        };

        let stop = context.stop().clone();
        let opened = self
            .tunnels
            .open(&target, remote, local, stop.clone())
            .await
            .map_err(open_failure)?;

        keep_hold_while_open(context, stop);

        Ok(TunnelInfo::from(opened))
    }
}

fn plan(resolved: Resolved, params: &OpenParams) -> Result<Resolved, ActionError> {
    let asked = destination(params)?;
    let local_port = local_port(params.local_port)?;

    match resolved {
        Resolved::Host(host) => {
            let remote = asked.ok_or_else(|| {
                ActionError::InvalidParams(
                    "remote_host and remote_port are needed unless the target is a tunnel resource"
                        .to_owned(),
                )
            })?;

            Ok(Resolved::Tunnel {
                host,
                remote,
                local: local_port,
            })
        }
        Resolved::Tunnel {
            host,
            remote,
            local,
        } => {
            check_matches_resource(asked.as_ref(), &remote, local_port, local)?;

            Ok(Resolved::Tunnel {
                host,
                remote,
                local: local.or(local_port),
            })
        }
        Resolved::Cluster { .. } => Err(ActionError::InvalidParams(
            "the target is a cluster; forward a port of it with k8s.port_forward".to_owned(),
        )),
    }
}

fn check_matches_resource(
    asked: Option<&Destination>,
    remote: &Destination,
    local_port: Option<u16>,
    local: Option<u16>,
) -> Result<(), ActionError> {
    if asked.is_some_and(|asked| asked != remote) {
        return Err(ActionError::InvalidParams(format!(
            "the tunnel resource forwards to {remote}; leave remote_host and remote_port out"
        )));
    }

    if let (Some(fixed), Some(asked)) = (local, local_port)
        && fixed != asked
    {
        return Err(ActionError::InvalidParams(format!(
            "the tunnel resource listens on local port {fixed}; leave local_port out"
        )));
    }

    Ok(())
}

fn destination(params: &OpenParams) -> Result<Option<Destination>, ActionError> {
    let (Some(remote_host), Some(remote_port)) = (&params.remote_host, params.remote_port) else {
        if params.remote_host.is_some() || params.remote_port.is_some() {
            return Err(ActionError::InvalidParams(
                "remote_host and remote_port go together".to_owned(),
            ));
        }

        return Ok(None);
    };

    let host = remote_host.trim();

    if host.is_empty() || host.contains(|character: char| character.is_whitespace()) {
        return Err(ActionError::InvalidParams(format!(
            "remote_host must be a host name or address, got {remote_host:?}"
        )));
    }

    if remote_port == 0 {
        return Err(ActionError::InvalidParams(
            "remote_port must be between 1 and 65535".to_owned(),
        ));
    }

    Ok(Some(Destination::new(host, remote_port)))
}

fn local_port(requested: Option<u16>) -> Result<Option<u16>, ActionError> {
    if requested == Some(0) {
        return Err(ActionError::InvalidParams(
            "local_port must be between 1 and 65535; leave it out to pick a free port".to_owned(),
        ));
    }

    Ok(requested)
}

fn open_failure(error: SshError) -> ActionError {
    match error {
        SshError::Timeout(limit) => ActionError::Failed(format!(
            "could not connect within {} s, so no tunnel was opened",
            limit.as_secs()
        )),
        error @ SshError::Listen { .. } => ActionError::Failed(error.to_string()),
        other => failure(other),
    }
}
