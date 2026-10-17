use russh_sftp::protocol::OpenFlags;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WriteMode {
    pub create: bool,
    pub truncate: bool,
}

impl From<WriteMode> for OpenFlags {
    fn from(mode: WriteMode) -> Self {
        let mut flags = Self::WRITE;

        if mode.create {
            flags |= Self::CREATE;
        }

        if mode.truncate {
            flags |= Self::TRUNCATE;
        }

        flags
    }
}
