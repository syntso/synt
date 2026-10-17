use crate::SyncError;

pub(super) const ID_BYTES: usize = 16;
pub(crate) const ID_LENGTH: usize = ID_BYTES * 2;

pub(super) fn encode(digest: &[u8]) -> String {
    let bytes = &digest[..ID_BYTES.min(digest.len())];

    data_encoding::HEXLOWER.encode(bytes)
}

pub(super) fn check(what: &'static str, text: &str) -> Result<(), SyncError> {
    let valid = text.len() == ID_LENGTH
        && text
            .chars()
            .all(|character| character.is_ascii_digit() || ('a'..='f').contains(&character));

    if valid {
        return Ok(());
    }

    Err(SyncError::BadId {
        what,
        text: text.to_owned(),
    })
}

macro_rules! hex_id {
    ($name:ident, $what:literal) => {
        #[derive(
            Debug,
            Clone,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            serde::Serialize,
            serde::Deserialize,
        )]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            pub fn parse(text: &str) -> Result<Self, $crate::SyncError> {
                Self::try_from(text.to_owned())
            }

            pub const LENGTH: usize = $crate::id::hex::ID_LENGTH;

            pub fn get(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = $crate::SyncError;

            fn try_from(text: String) -> Result<Self, $crate::SyncError> {
                $crate::id::hex::check($what, &text)?;

                Ok(Self(text))
            }
        }

        impl From<$name> for String {
            fn from(id: $name) -> Self {
                id.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

pub(super) use hex_id;

#[cfg(test)]
mod tests {
    use super::{check, encode};

    #[test]
    fn an_id_is_the_first_sixteen_bytes_in_lowercase_hex() {
        let digest = [0xab_u8; 32];

        assert_eq!(encode(&digest), "ab".repeat(16));
    }

    #[test]
    fn only_thirty_two_lowercase_hex_characters_pass() {
        assert!(check("revision", &"0f".repeat(16)).is_ok());

        for text in [
            "",
            "0f",
            &"0F".repeat(16),
            &"0g".repeat(16),
            &"0f".repeat(17),
        ] {
            assert!(check("revision", text).is_err(), "{text:?}");
        }
    }
}
