use super::{LoginStatus, Program};
use crate::PlatformError;

pub(super) trait Registration: Sized {
    fn for_this_user() -> Result<Self, PlatformError>;

    fn install(&self, program: &Program) -> Result<(), PlatformError>;

    fn uninstall(&self) -> Result<bool, PlatformError>;

    fn status(&self) -> Result<LoginStatus, PlatformError>;
}
