use crate::seal::Keyring;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Standing {
    Member(Keyring),
    Waiting,
    Removed,
}
