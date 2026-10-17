use std::path::Path;

use russh::keys::PublicKey;

use crate::config::UserConfig;
use crate::known_hosts::KnownHosts;

pub fn known_hosts(text: &str, host: &str, port: u16, presented: &str) -> usize {
    let known = KnownHosts::parse(text);
    let algorithms = known.known_algorithms(host, port);

    if let Ok(key) = PublicKey::from_openssh(presented) {
        known.verdict(host, port, &key);
    }

    algorithms.len()
}

pub fn config(text: &[u8], host: &str) -> Option<Vec<String>> {
    let config = UserConfig::parse(Path::new("config"), text).ok()?;
    let settings = config.settings(host);

    let mut seen = config.aliases();
    seen.extend(settings.proxy_jump);

    Some(seen)
}
