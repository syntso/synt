mod database;

pub(crate) use database::ResolvedDatabase;

use std::fmt;
use std::num::NonZeroU16;

use synt_kube::ClusterSpec;
use synt_ssh::{Destination, Target};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Resolved {
    Host(Target),
    Tunnel {
        host: Target,
        remote: Destination,
        local: Option<NonZeroU16>,
    },
    Cluster {
        spec: ClusterSpec,
        via: Option<Target>,
    },
}

impl Resolved {
    pub(crate) fn host(&self) -> Option<&Target> {
        match self {
            Self::Host(host) | Self::Tunnel { host, .. } => Some(host),
            Self::Cluster { via, .. } => via.as_ref(),
        }
    }

    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Self::Host(_) => "host",
            Self::Tunnel { .. } => "tunnel",
            Self::Cluster { .. } => "cluster",
        }
    }
}

impl fmt::Display for Resolved {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Host(host) => write!(formatter, "{}", host.route()),
            Self::Tunnel { host, remote, .. } => {
                write!(formatter, "{} forwarding to {remote}", host.route())
            }
            Self::Cluster { spec, via } => {
                let context = spec.context.as_deref().unwrap_or("the current context");
                write!(formatter, "{context} in {}", spec.kubeconfig)?;

                match via {
                    Some(via) => write!(formatter, " via {}", via.route()),
                    None => Ok(()),
                }
            }
        }
    }
}
