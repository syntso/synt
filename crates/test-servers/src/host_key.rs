use data_encoding::{BASE64, BASE64_NOPAD};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostKey {
    algorithm: String,
    blob: String,
}

impl HostKey {
    pub fn parse(public_key_line: &str) -> Option<Self> {
        let mut words = public_key_line.split_whitespace();

        let algorithm = words.next()?.to_owned();
        let blob = words.next()?.to_owned();

        BASE64.decode(blob.as_bytes()).ok()?;

        Some(Self { algorithm, blob })
    }

    pub fn known_hosts_line(&self, host: &str) -> String {
        format!("{host} {} {}", self.algorithm, self.blob)
    }

    pub fn fingerprint(&self) -> String {
        let bytes = BASE64
            .decode(self.blob.as_bytes())
            .expect("the blob was checked when the key was parsed");
        let digest = Sha256::digest(&bytes);

        format!("SHA256:{}", BASE64_NOPAD.encode(&digest))
    }
}

#[cfg(test)]
mod tests {
    use super::HostKey;

    const LINE: &str =
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl bastion";

    #[test]
    fn a_public_key_line_becomes_a_known_hosts_line() {
        let key = HostKey::parse(LINE).unwrap();

        assert_eq!(
            key.known_hosts_line("[127.0.0.1]:2222"),
            "[127.0.0.1]:2222 ssh-ed25519 \
             AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl"
        );
    }

    #[test]
    fn the_fingerprint_is_the_openssh_sha256_form() {
        let key = HostKey::parse(LINE).unwrap();

        assert_eq!(
            key.fingerprint(),
            "SHA256:+DiY3wvvV6TuJJhbpZisF/zLDA0zPMSvHdkr4UvCOqU"
        );
    }

    #[test]
    fn text_that_is_not_a_key_is_refused() {
        assert_eq!(HostKey::parse("ssh-ed25519 not*base64"), None);
        assert_eq!(HostKey::parse(""), None);
    }
}
