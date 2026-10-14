use crate::SyncError;

pub(crate) fn random_bytes<const N: usize>() -> Result<[u8; N], SyncError> {
    let mut bytes = [0_u8; N];

    getrandom::fill(&mut bytes).map_err(|error| SyncError::Random(error.to_string()))?;

    Ok(bytes)
}
