use std::io;

use serde_json::Value;
use synt_journal::{Actor, Name};
use synt_platform::{AppCheck, Peer};
use synt_proto::{ClientKind, Hello, ProtocolVersion, RpcError, Welcome};

use crate::VERSION;
use crate::broker::requester;
use crate::log::log;

const UNVERIFIED_APP: &str = "unverified-app";

pub(in crate::server) struct Greeted {
    pub(in crate::server) actor: Actor,
    pub(in crate::server) welcome: Value,
}

impl Greeted {
    pub(super) async fn welcome(
        hello: Hello,
        peer: io::Result<Peer>,
        apps: &AppCheck,
    ) -> Result<Self, RpcError> {
        let actor = match hello.client {
            ClientKind::App => Self::app(peer, apps).await?,
            ClientKind::Agent { name } => Self::agent(name)?,
        };

        let welcome = Welcome {
            protocol: ProtocolVersion::CURRENT,
            core: VERSION.to_owned(),
            actor: requester(&actor),
        };
        let welcome = serde_json::to_value(welcome)
            .map_err(|_| RpcError::internal("the welcome cannot be encoded"))?;

        Ok(Self { actor, welcome })
    }

    pub(super) fn agent(name: String) -> Result<Actor, RpcError> {
        Name::new(name)
            .map(Actor::Agent)
            .map_err(|error| RpcError::invalid_params(format!("agent name: {error}")))
    }

    async fn app(peer: io::Result<Peer>, apps: &AppCheck) -> Result<Actor, RpcError> {
        let verdict = match peer {
            Ok(peer) => apps.verify(peer).await.map_err(|error| error.to_string()),
            Err(error) => Err(format!(
                "the system cannot say which process says it is the app: {error}"
            )),
        };

        let Err(reason) = verdict else {
            return Ok(Actor::Person);
        };

        log!("{reason}; it acts as the agent {UNVERIFIED_APP} and cannot answer approvals");

        Name::new(UNVERIFIED_APP)
            .map(Actor::Agent)
            .map_err(|_| RpcError::internal("the unverified app name is invalid"))
    }
}
