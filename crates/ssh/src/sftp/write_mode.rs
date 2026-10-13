#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WriteMode {
    pub create: bool,
    pub truncate: bool,
}
