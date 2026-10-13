use synt_ssh::{Destination, Target};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Resolved {
    Host(Target),
    Tunnel {
        host: Target,
        remote: Destination,
        local: Option<u16>,
    },
}

impl Resolved {
    pub(crate) fn host(&self) -> &Target {
        match self {
            Self::Host(host) | Self::Tunnel { host, .. } => host,
        }
    }

    pub(crate) fn describe(&self) -> String {
        match self {
            Self::Host(host) => login(host),
            Self::Tunnel { host, remote, .. } => {
                format!("{} forwarding to {}", login(host), remote)
            }
        }
    }
}

fn login(target: &Target) -> String {
    let place = format!("{}@{}:{}", target.user(), target.host(), target.port());

    if target.jumps().is_empty() {
        return place;
    }

    let jumps: Vec<String> = target.jumps().iter().map(login).collect();

    format!("{place} via [{}]", jumps.join(", "))
}
