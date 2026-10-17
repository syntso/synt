use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

const LOCALHOST: &str = "localhost";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Address(IpAddr);

impl Address {
    pub(super) fn new(address: IpAddr) -> Self {
        Self(address)
    }

    pub(super) fn reachable_as(self, host: &str) -> bool {
        let literal = host
            .parse::<IpAddr>()
            .is_ok_and(|literal| literal == self.0);

        if literal {
            return true;
        }

        if host == LOCALHOST {
            return self.0.is_loopback();
        }

        self.is_public()
    }

    pub(super) fn refusal(self, host: &str) -> ureq::Error {
        ureq::Error::Io(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "{host} points at {}, an address on this computer or its network; \
                 an extension reaches such an address only when capabilities.http lists it",
                self.0
            ),
        ))
    }

    fn is_public(self) -> bool {
        match self.0 {
            IpAddr::V4(address) => Self::is_public_v4(address),
            IpAddr::V6(address) => match address.to_ipv4_mapped() {
                Some(mapped) => Self::is_public_v4(mapped),
                None => Self::is_public_v6(address),
            },
        }
    }

    fn is_public_v4(address: Ipv4Addr) -> bool {
        let [first, second, ..] = address.octets();
        let shared = first == 100 && (64..128).contains(&second);
        let this_network = first == 0;

        !(address.is_loopback()
            || address.is_private()
            || address.is_link_local()
            || address.is_unspecified()
            || address.is_broadcast()
            || shared
            || this_network)
    }

    fn is_public_v6(address: Ipv6Addr) -> bool {
        let first = address.segments()[0];
        let unique_local = first & 0xfe00 == 0xfc00;
        let link_local = first & 0xffc0 == 0xfe80;
        let site_local = first & 0xffc0 == 0xfec0;

        !(address.is_loopback()
            || address.is_unspecified()
            || unique_local
            || link_local
            || site_local)
    }
}
