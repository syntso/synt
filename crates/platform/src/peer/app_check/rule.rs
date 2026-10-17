use std::fmt::{Debug, Display};

use crate::{Peer, PlatformError};

pub(super) trait Rule: Debug + Display + Send + Sync {
    fn weakness(&self) -> Option<&'static str>;

    fn check(&self, peer: &Peer) -> Result<(), PlatformError>;
}
