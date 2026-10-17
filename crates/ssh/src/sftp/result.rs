use russh_sftp::client::error::Error;
use russh_sftp::protocol::StatusCode;

use crate::SshError;

pub(super) trait SftpResult<T> {
    fn during(self, operation: &'static str, path: &str) -> Result<T, SshError>;

    fn or_end(self) -> Result<Option<T>, Error>;
}

impl<T> SftpResult<T> for Result<T, Error> {
    fn during(self, operation: &'static str, path: &str) -> Result<T, SshError> {
        self.map_err(|error| SshError::sftp(operation, path, &error))
    }

    fn or_end(self) -> Result<Option<T>, Error> {
        match self {
            Ok(value) => Ok(Some(value)),
            Err(Error::Status(status)) if status.status_code == StatusCode::Eof => Ok(None),
            Err(error) => Err(error),
        }
    }
}
