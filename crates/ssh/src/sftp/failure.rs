use std::io;

use russh_sftp::client::error::Error;

use crate::SshError;

const CLOSED: &str = "session closed";
const NOT_SENT: &str = "SendError";

pub(super) fn failure(operation: &'static str, path: &str) -> impl FnOnce(Error) -> SshError {
    let path = path.to_owned();

    move |error| classify(operation, path, &error)
}

pub(super) fn io_failure(
    operation: &'static str,
    path: &str,
) -> impl FnOnce(io::Error) -> SshError {
    let path = path.to_owned();

    move |error| match error
        .get_ref()
        .and_then(|inner| inner.downcast_ref::<Error>())
    {
        Some(inner) => classify(operation, path, inner),
        None => SshError::Sftp {
            operation,
            path,
            reason: error.to_string(),
        },
    }
}

fn classify(operation: &'static str, path: String, error: &Error) -> SshError {
    match error {
        Error::Status(status) => SshError::Sftp {
            operation,
            path,
            reason: status_reason(
                status.error_message.as_str(),
                &status.status_code.to_string(),
            ),
        },
        Error::UnexpectedBehavior(message) if is_unsent(message) => SshError::SftpClosed,
        Error::Timeout => SshError::SftpLost {
            operation,
            path,
            reason: "the server did not answer in time".to_owned(),
        },
        Error::IO(_) | Error::UnexpectedBehavior(_) => SshError::SftpLost {
            operation,
            path,
            reason: error.to_string(),
        },
        Error::Limited(_) | Error::UnexpectedPacket => SshError::Sftp {
            operation,
            path,
            reason: error.to_string(),
        },
    }
}

fn status_reason(message: &str, code: &str) -> String {
    if message.is_empty() {
        return code.to_owned();
    }

    message.to_owned()
}

fn is_unsent(message: &str) -> bool {
    message == CLOSED || message.starts_with(NOT_SENT)
}
