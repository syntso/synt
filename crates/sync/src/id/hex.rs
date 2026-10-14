use crate::SyncError;

pub(crate) const ID_BYTES: usize = 16;
const ID_LENGTH: usize = ID_BYTES * 2;

pub(crate) fn hex_id(digest: &[u8]) -> String {
    let bytes = &digest[..ID_BYTES.min(digest.len())];

    data_encoding::HEXLOWER.encode(bytes)
}

pub(crate) fn check_hex_id(what: &str, text: &str) -> Result<(), SyncError> {
    let valid = text.len() == ID_LENGTH
        && text
            .chars()
            .all(|character| character.is_ascii_digit() || ('a'..='f').contains(&character));

    if valid {
        return Ok(());
    }

    Err(SyncError::invalid(format!(
        "{text:?} is not a {what}: it needs {ID_LENGTH} lowercase hex characters"
    )))
}

#[cfg(test)]
mod tests {
    use super::{check_hex_id, hex_id};

    #[test]
    fn an_id_is_the_first_sixteen_bytes_in_lowercase_hex() {
        let digest = [0xab_u8; 32];

        assert_eq!(hex_id(&digest), "ab".repeat(16));
    }

    #[test]
    fn only_thirty_two_lowercase_hex_characters_pass() {
        assert!(check_hex_id("revision", &"0f".repeat(16)).is_ok());

        for text in [
            "",
            "0f",
            &"0F".repeat(16),
            &"0g".repeat(16),
            &"0f".repeat(17),
        ] {
            assert!(check_hex_id("revision", text).is_err(), "{text:?}");
        }
    }
}
