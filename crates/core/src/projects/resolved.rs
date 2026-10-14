use synt_kube::ClusterSpec;
use synt_ssh::{Destination, Target};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Resolved {
    Host(Target),
    Tunnel {
        host: Target,
        remote: Destination,
        local: Option<u16>,
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

    pub(crate) fn describe(&self) -> String {
        match self {
            Self::Host(host) => login_label(host),
            Self::Tunnel { host, remote, .. } => {
                format!("{} forwarding to {}", login_label(host), remote)
            }
            Self::Cluster { spec, via } => cluster(spec, via.as_ref()),
        }
    }
}

pub(crate) fn login_label(target: &Target) -> String {
    let place = format!("{}@{}:{}", target.user(), target.host(), target.port());

    if target.jumps().is_empty() {
        return place;
    }

    let jumps: Vec<String> = target.jumps().iter().map(login_label).collect();

    format!("{place} via [{}]", jumps.join(", "))
}

fn cluster(spec: &ClusterSpec, via: Option<&Target>) -> String {
    let context = spec.context.as_deref().unwrap_or("the current context");
    let place = format!("{context} in {}", spec.kubeconfig);

    match via {
        Some(via) => format!("{place} via {}", login_label(via)),
        None => place,
    }
}
