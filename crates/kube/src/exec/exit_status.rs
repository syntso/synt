use k8s_openapi::apimachinery::pkg::apis::meta::v1::Status;

const SUCCESS: &str = "Success";
const NON_ZERO_EXIT: &str = "NonZeroExitCode";
const EXIT_CODE: &str = "ExitCode";

pub(crate) enum ExitStatus {
    Exited(i32),
    Failed(String),
}

impl ExitStatus {
    pub(crate) fn from_status(status: Option<Status>) -> Self {
        let Some(status) = status else {
            return Self::Failed("the cluster reported no exit status".to_owned());
        };

        if status.status.as_deref() == Some(SUCCESS) {
            return Self::Exited(0);
        }

        if status.reason.as_deref() == Some(NON_ZERO_EXIT)
            && let Some(code) = exit_code(&status)
        {
            return Self::Exited(code);
        }

        Self::Failed(
            status
                .message
                .unwrap_or_else(|| "the command failed without a reason".to_owned()),
        )
    }
}

fn exit_code(status: &Status) -> Option<i32> {
    status
        .details
        .as_ref()?
        .causes
        .as_ref()?
        .iter()
        .find(|cause| cause.reason.as_deref() == Some(EXIT_CODE))?
        .message
        .as_ref()?
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::{Status, StatusCause, StatusDetails};

    use super::ExitStatus;

    fn failure(reason: &str, message: &str, causes: Vec<StatusCause>) -> Status {
        Status {
            status: Some("Failure".to_owned()),
            reason: Some(reason.to_owned()),
            message: Some(message.to_owned()),
            details: Some(StatusDetails {
                causes: Some(causes),
                ..StatusDetails::default()
            }),
            ..Status::default()
        }
    }

    #[test]
    fn success_is_exit_zero() {
        let status = Status {
            status: Some("Success".to_owned()),
            ..Status::default()
        };

        assert!(matches!(
            ExitStatus::from_status(Some(status)),
            ExitStatus::Exited(0)
        ));
    }

    #[test]
    fn a_non_zero_exit_carries_its_code() {
        let cause = StatusCause {
            reason: Some("ExitCode".to_owned()),
            message: Some("3".to_owned()),
            ..StatusCause::default()
        };
        let status = failure(
            "NonZeroExitCode",
            "command terminated with exit code 3",
            vec![cause],
        );

        assert!(matches!(
            ExitStatus::from_status(Some(status)),
            ExitStatus::Exited(3)
        ));
    }

    #[test]
    fn a_command_that_could_not_start_is_a_failure_with_the_reason() {
        let status = failure(
            "InternalError",
            "executable file not found in $PATH",
            Vec::new(),
        );

        let ExitStatus::Failed(reason) = ExitStatus::from_status(Some(status)) else {
            panic!("a failure");
        };
        assert!(reason.contains("not found"));
    }

    #[test]
    fn no_status_at_all_is_a_failure() {
        assert!(matches!(
            ExitStatus::from_status(None),
            ExitStatus::Failed(_)
        ));
    }
}
