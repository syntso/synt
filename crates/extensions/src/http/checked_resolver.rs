use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use ureq::config::Config;
use ureq::http::Uri;
use ureq::unversioned::resolver::{DefaultResolver, ResolvedSocketAddrs, Resolver};
use ureq::unversioned::transport::NextTimeout;

const LOCALHOST: &str = "localhost";

#[derive(Debug)]
pub(crate) struct CheckedResolver<R = DefaultResolver> {
    inner: R,
}

impl CheckedResolver {
    pub(crate) fn new() -> Self {
        Self {
            inner: DefaultResolver::default(),
        }
    }
}

impl<R: Resolver> CheckedResolver<R> {
    #[cfg(test)]
    pub(crate) fn over(inner: R) -> Self {
        Self { inner }
    }

    fn checked(
        &self,
        host: &str,
        found: &ResolvedSocketAddrs,
    ) -> Result<ResolvedSocketAddrs, ureq::Error> {
        let mut kept = self.inner.empty();
        let mut refused = None;

        for address in found {
            if may_reach(host, address.ip()) {
                kept.push(*address);
            } else {
                refused.get_or_insert(address.ip());
            }
        }

        match refused {
            Some(address) if kept.is_empty() => Err(refusal(host, address)),
            _ => Ok(kept),
        }
    }
}

impl<R: Resolver> Resolver for CheckedResolver<R> {
    fn resolve(
        &self,
        uri: &Uri,
        config: &Config,
        timeout: NextTimeout,
    ) -> Result<ResolvedSocketAddrs, ureq::Error> {
        let host = uri.host().ok_or(ureq::Error::HostNotFound)?;
        let found = self.inner.resolve(uri, config, timeout)?;

        let kept = self.checked(&host.to_ascii_lowercase(), &found)?;
        if kept.is_empty() {
            return Err(ureq::Error::HostNotFound);
        }

        Ok(kept)
    }
}

fn may_reach(host: &str, address: IpAddr) -> bool {
    if host
        .parse::<IpAddr>()
        .is_ok_and(|literal| literal == address)
    {
        return true;
    }

    if host == LOCALHOST {
        return address.is_loopback();
    }

    is_public(address)
}

fn is_public(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => is_public_v4(address),
        IpAddr::V6(address) => match address.to_ipv4_mapped() {
            Some(mapped) => is_public_v4(mapped),
            None => is_public_v6(address),
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

    !(address.is_loopback() || address.is_unspecified() || unique_local || link_local || site_local)
}

fn refusal(host: &str, address: IpAddr) -> ureq::Error {
    ureq::Error::Io(io::Error::new(
        io::ErrorKind::PermissionDenied,
        format!(
            "{host} points at {address}, an address on this computer or its network; \
             an extension reaches such an address only when capabilities.http lists it"
        ),
    ))
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, SocketAddr};

    use ureq::config::Config;
    use ureq::http::Uri;
    use ureq::unversioned::resolver::{ResolvedSocketAddrs, Resolver};
    use ureq::unversioned::transport::NextTimeout;

    use super::{CheckedResolver, may_reach};

    #[derive(Debug)]
    struct Pointing(Vec<&'static str>);

    impl Resolver for Pointing {
        fn resolve(
            &self,
            _uri: &Uri,
            _config: &Config,
            _timeout: NextTimeout,
        ) -> Result<ResolvedSocketAddrs, ureq::Error> {
            let mut found = self.empty();

            for address in &self.0 {
                found.push(SocketAddr::new(address.parse().unwrap(), 443));
            }

            Ok(found)
        }
    }

    fn resolved(url: &str, addresses: Vec<&'static str>) -> Result<Vec<SocketAddr>, String> {
        let resolver = CheckedResolver::over(Pointing(addresses));
        let uri: Uri = url.parse().unwrap();
        let timeout = NextTimeout {
            after: ureq::unversioned::transport::time::Duration::NotHappening,
            reason: ureq::Timeout::Global,
        };

        resolver
            .resolve(&uri, &Config::default(), timeout)
            .map(|found| found.iter().copied().collect())
            .map_err(|error| error.to_string())
    }

    fn reaches(host: &str, address: &str) -> bool {
        may_reach(host, address.parse::<IpAddr>().unwrap())
    }

    #[test]
    fn a_name_that_points_at_this_computer_is_refused() {
        let refused = resolved("https://api.vendor.test/", vec!["127.0.0.1"]).unwrap_err();

        assert!(
            refused.contains("api.vendor.test points at 127.0.0.1"),
            "{refused}"
        );
    }

    #[test]
    fn a_name_keeps_only_its_public_addresses() {
        let kept = resolved("https://api.vendor.test/", vec!["10.0.0.7", "203.0.113.9"]).unwrap();

        assert_eq!(kept, vec!["203.0.113.9:443".parse().unwrap()]);
    }

    #[test]
    fn names_never_reach_local_network_or_metadata_addresses() {
        for address in [
            "127.0.0.1",
            "10.1.2.3",
            "172.16.0.1",
            "192.168.1.1",
            "169.254.169.254",
            "100.64.0.1",
            "0.0.0.0",
            "::1",
            "::",
            "fd00::1",
            "fe80::1",
            "::ffff:127.0.0.1",
            "::ffff:169.254.169.254",
        ] {
            assert!(!reaches("api.vendor.test", address), "{address}");
        }

        for address in ["203.0.113.9", "8.8.8.8", "2001:db8::1", "100.128.0.1"] {
            assert!(reaches("api.vendor.test", address), "{address}");
        }
    }

    #[test]
    fn a_listed_address_or_localhost_reaches_this_computer() {
        assert!(reaches("127.0.0.1", "127.0.0.1"));
        assert!(reaches("10.0.0.7", "10.0.0.7"));
        assert!(reaches("localhost", "127.0.0.1"));
        assert!(reaches("localhost", "::1"));
        assert!(!reaches("localhost", "10.0.0.7"));

        assert_eq!(
            resolved("http://localhost:3000/", vec!["127.0.0.1"]).unwrap(),
            vec!["127.0.0.1:443".parse().unwrap()]
        );
    }
}
