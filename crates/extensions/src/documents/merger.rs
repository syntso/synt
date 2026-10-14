use crate::interface::GuestError;

pub(crate) trait Merger {
    fn merge(
        &self,
        kind: &str,
        sides: (Option<&[u8]>, &[u8], &[u8]),
    ) -> Result<Vec<u8>, GuestError>;
}
