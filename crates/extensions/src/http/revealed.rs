use zeroize::Zeroizing;

use super::SecretHeader;

pub(crate) struct Revealed {
    header: String,
    prefix: String,
    reference: String,
    value: Zeroizing<Vec<u8>>,
}

impl Revealed {
    pub(crate) fn new(attached: &SecretHeader, value: Zeroizing<Vec<u8>>) -> Self {
        Self {
            header: attached.name.clone(),
            prefix: attached.prefix.clone(),
            reference: format!("secret://{}", attached.secret),
            value,
        }
    }

    pub(crate) fn header(&self) -> &str {
        &self.header
    }

    pub(crate) fn header_value(&self) -> Result<Zeroizing<String>, String> {
        let text = std::str::from_utf8(&self.value)
            .map_err(|_| format!("{} is not text and cannot go in a header", self.reference))?;

        if text.is_empty() || text.contains(char::is_control) {
            return Err(format!(
                "{} is empty or holds control characters and cannot go in a header",
                self.reference
            ));
        }

        Ok(Zeroizing::new(format!("{}{text}", self.prefix)))
    }

    pub(crate) fn scrub(&self, bytes: &[u8]) -> Vec<u8> {
        let needle: &[u8] = &self.value;

        if needle.is_empty() || needle.len() > bytes.len() {
            return bytes.to_vec();
        }

        let mut scrubbed = Vec::with_capacity(bytes.len());
        let mut rest = bytes;

        while let Some(at) = find(rest, needle) {
            scrubbed.extend_from_slice(&rest[..at]);
            scrubbed.extend_from_slice(self.reference.as_bytes());
            rest = &rest[at + needle.len()..];
        }

        scrubbed.extend_from_slice(rest);
        scrubbed
    }

    pub(crate) fn scrub_text(&self, text: &str) -> String {
        String::from_utf8_lossy(&self.scrub(text.as_bytes())).into_owned()
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use zeroize::Zeroizing;

    use super::Revealed;
    use crate::http::SecretHeader;

    fn attached(secret: &str) -> SecretHeader {
        SecretHeader {
            name: "Authorization".to_owned(),
            secret: secret.to_owned(),
            prefix: "Bearer ".to_owned(),
        }
    }

    fn token() -> Revealed {
        Revealed::new(
            &attached("grafana/token"),
            Zeroizing::new(b"s3cr3t".to_vec()),
        )
    }

    #[test]
    fn every_copy_of_the_value_is_replaced_by_its_reference() {
        let echoed = b"Authorization: Bearer s3cr3t; again s3cr3ts3cr3t";

        let scrubbed = token().scrub(echoed);

        assert_eq!(
            String::from_utf8(scrubbed).unwrap(),
            "Authorization: Bearer secret://grafana/token; again \
             secret://grafana/tokensecret://grafana/token"
        );
    }

    #[test]
    fn text_without_the_value_is_left_alone() {
        assert_eq!(token().scrub_text("nothing here"), "nothing here");
        assert_eq!(token().scrub(b"s3c"), b"s3c");
    }

    #[test]
    fn a_header_value_is_the_prefix_and_the_secret() {
        assert_eq!(*token().header_value().unwrap(), "Bearer s3cr3t");
        assert_eq!(token().header(), "Authorization");

        let binary = Revealed::new(&attached("blob"), Zeroizing::new(vec![0, 159, 146, 150]));
        assert!(binary.header_value().is_err());
    }
}
