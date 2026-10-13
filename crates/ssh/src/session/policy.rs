use std::borrow::Cow;

use russh::keys::{Algorithm, EcdsaCurve, HashAlg};
use russh::{Preferred, cipher, compression, kex, mac};

use crate::known_hosts::same_family;

const KEY_EXCHANGES: &[kex::Name] = &[
    kex::MLKEM768X25519_SHA256,
    kex::CURVE25519,
    kex::CURVE25519_PRE_RFC_8731,
    kex::EXTENSION_SUPPORT_AS_CLIENT,
    kex::EXTENSION_OPENSSH_STRICT_KEX_AS_CLIENT,
];

const CIPHERS: &[cipher::Name] = &[
    cipher::AES_256_GCM,
    cipher::AES_128_GCM,
    cipher::CHACHA20_POLY1305,
    cipher::AES_256_CTR,
    cipher::AES_128_CTR,
];

const MACS: &[mac::Name] = &[mac::HMAC_SHA256_ETM, mac::HMAC_SHA512_ETM];

const COMPRESSION: &[compression::Name] = &[compression::NONE];

const HOST_KEYS: [Algorithm; 5] = [
    Algorithm::Ed25519,
    Algorithm::Ecdsa {
        curve: EcdsaCurve::NistP256,
    },
    Algorithm::Ecdsa {
        curve: EcdsaCurve::NistP384,
    },
    Algorithm::Rsa {
        hash: Some(HashAlg::Sha512),
    },
    Algorithm::Rsa {
        hash: Some(HashAlg::Sha256),
    },
];

pub(super) fn preferred(known: &[Algorithm]) -> Preferred {
    Preferred {
        kex: Cow::Borrowed(KEY_EXCHANGES),
        key: Cow::Owned(known_first(known, &HOST_KEYS)),
        host_key_certificates: Cow::Borrowed(&[]),
        cipher: Cow::Borrowed(CIPHERS),
        mac: Cow::Borrowed(MACS),
        compression: Cow::Borrowed(COMPRESSION),
    }
}

fn known_first(known: &[Algorithm], offered: &[Algorithm]) -> Vec<Algorithm> {
    let is_known =
        |algorithm: &&Algorithm| known.iter().any(|pinned| same_family(pinned, algorithm));

    let (first, rest): (Vec<&Algorithm>, Vec<&Algorithm>) = offered.iter().partition(is_known);

    first.into_iter().chain(rest).cloned().collect()
}
