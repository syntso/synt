mod sides;

pub use sides::MergeSides;

use crate::interface::GuestError;

pub(crate) trait Merger {
    fn merge(&self, kind: &str, sides: MergeSides<'_>) -> Result<Vec<u8>, GuestError>;
}
