mod params;

use serde_json::Value;
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_ssh::Tunnels;

pub use params::OpenParams;

use super::TunnelInfo;
use crate::CoreError;
use crate::builtin::hold::keep_hold_while_open;
use crate::projects::{Projects, Resolved};

#[derive(Debug, Clone)]
pub struct TunnelOpen {
    tunnels: Tunnels,
    projects: Projects,
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

        params.plan(resolved).map(drop)
    }

    fn risk(&self, target: Option<&ResourceRef>, _params: &OpenParams) -> Risk {
        self.projects.login_risk(target, Self::RISK)
    }

    fn destination(&self, target: Option<&ResourceRef>, params: &OpenParams) -> Option<String> {
        let target = target?;
        let resolved = self.projects.resolve(target.as_str()).ok()?;
        let planned = params.plan(resolved).ok()?;

        Some(self.projects.destination(target.as_str(), &planned))
    }

    fn summary(&self, output: &TunnelInfo) -> Value {
        output.ports()
    }

    async fn run(&self, context: &Context, params: OpenParams) -> Result<TunnelInfo, ActionError> {
        let host = context.target().ok_or(ActionError::TargetRequired)?;

        let resolved = self.projects.resolve_any(host.as_str()).await?;
        let planned = params.plan(resolved)?;
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
            .map_err(CoreError::from)?;

        keep_hold_while_open(context, stop);

        Ok(TunnelInfo::from(opened))
    }
}
