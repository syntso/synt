use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use crate::endpoint::Endpoint;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Place {
    Network {
        host: String,
        port: u16,
        reached_at: Option<SocketAddr>,
    },
    File(PathBuf),
}

impl Place {
    pub(crate) fn host(&self) -> Option<&str> {
        match self {
            Self::Network { host, .. } => Some(host),
            Self::File(_) => None,
        }
    }

    pub(crate) fn file(&self) -> Option<&Path> {
        match self {
            Self::File(path) => Some(path),
            Self::Network { .. } => None,
        }
    }

    pub(crate) fn dial(&self) -> Option<Endpoint> {
        match self {
            Self::Network {
                reached_at: Some(address),
                ..
            } => Some(Endpoint::new(address.ip().to_string(), address.port())),
            Self::Network { host, port, .. } => Some(Endpoint::new(host.clone(), *port)),
            Self::File(_) => None,
        }
    }

    pub(crate) fn is_tunneled(&self) -> bool {
        matches!(
            self,
            Self::Network {
                reached_at: Some(_),
                ..
            }
        )
    }

    pub(crate) fn reached_at(&mut self, address: SocketAddr) {
        if let Self::Network { reached_at, .. } = self {
            *reached_at = Some(address);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::Place;
    use crate::endpoint::Endpoint;

    fn network() -> Place {
        Place::Network {
            host: "orders-db.internal".to_owned(),
            port: 3306,
            reached_at: None,
        }
    }

    #[test]
    fn a_tunnel_changes_where_to_dial_but_not_the_host_name() {
        let mut place = network();

        place.reached_at("127.0.0.1:40123".parse().expect("address"));

        assert_eq!(place.dial(), Some(Endpoint::new("127.0.0.1", 40123)));
        assert_eq!(place.host(), Some("orders-db.internal"));
        assert!(place.is_tunneled());
    }

    #[test]
    fn a_direct_place_dials_its_own_host() {
        let place = network();

        assert_eq!(place.dial(), Some(Endpoint::new("orders-db.internal", 3306)));
        assert!(!place.is_tunneled());
    }

    #[test]
    fn a_file_has_no_host_to_dial() {
        let place = Place::File("shop.sqlite".into());

        assert_eq!(place.dial(), None);
        assert_eq!(place.file(), Some(Path::new("shop.sqlite")));
    }
}
