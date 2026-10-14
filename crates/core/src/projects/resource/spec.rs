use crate::CoreError;

pub(super) trait ResourceSpec {
    fn kind(&self) -> &'static str;

    fn check(&self) -> Result<(), CoreError>;
}
