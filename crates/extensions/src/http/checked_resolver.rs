mod address;

use ureq::config::Config;
use ureq::http::Uri;
use ureq::unversioned::resolver::{DefaultResolver, ResolvedSocketAddrs, Resolver};
use ureq::unversioned::transport::NextTimeout;

use address::Address;

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
            let checked = Address::new(address.ip());

            if checked.reachable_as(host) {
                kept.push(*address);
            } else {
                refused.get_or_insert(checked);
            }
        }

        match refused {
            Some(address) if kept.is_empty() => Err(address.refusal(host)),
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

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, SocketAddr};

    use ureq::config::Config;
    use ureq::http::Uri;
    use ureq::unversioned::resolver::{ResolvedSocketAddrs, Resolver};
    use ureq::unversioned::transport::NextTimeout;

    use super::{Address, CheckedResolver};

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
        Address::new(address.parse::<IpAddr>().unwrap()).reachable_as(host)
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
