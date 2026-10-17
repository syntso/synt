use std::time::Duration;

use russh::client::Config;

use super::policy::preferred;
use crate::Target;
use crate::known_hosts::KnownHosts;

const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(15);
const KEEPALIVE_MISSES: usize = 3;
const WINDOW_BYTES: u32 = 2 * 1024 * 1024;
const PACKET_BYTES: u32 = 32 * 1024;

pub(crate) fn client_config(known_hosts: &KnownHosts, hop: &Target) -> Config {
    let known = known_hosts.known_algorithms(hop.host(), hop.port());

    Config {
        nodelay: true,
        keepalive_interval: Some(KEEPALIVE_INTERVAL),
        keepalive_max: KEEPALIVE_MISSES,
        window_size: WINDOW_BYTES,
        maximum_packet_size: PACKET_BYTES,
        preferred: preferred(&known),
        ..Config::default()
    }
}
