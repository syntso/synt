use std::time::Duration;

use schemars::JsonSchema;
use serde::Deserialize;

use crate::KubeError;

const LONGEST_TIMEOUT: Duration = Duration::from_hours(24);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(try_from = "Option<u64>")]
pub struct ExecLimits {
    pub timeout: Duration,
    pub output_bytes: usize,
}

impl Default for ExecLimits {
    fn default() -> Self {
        Self {
            timeout: Duration::from_mins(10),
            output_bytes: 1024 * 1024,
        }
    }
}

impl TryFrom<Option<u64>> for ExecLimits {
    type Error = KubeError;

    fn try_from(timeout_seconds: Option<u64>) -> Result<Self, KubeError> {
        let Some(seconds) = timeout_seconds else {
            return Ok(Self::default());
        };

        let timeout = Duration::from_secs(seconds);

        if timeout.is_zero() || timeout > LONGEST_TIMEOUT {
            return Err(KubeError::invalid(format!(
                "timeout_seconds must be between 1 and {}, got {seconds}",
                LONGEST_TIMEOUT.as_secs()
            )));
        }

        Ok(Self {
            timeout,
            ..Self::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::ExecLimits;

    #[test]
    fn no_timeout_keeps_the_default_limits() {
        assert_eq!(ExecLimits::try_from(None).unwrap(), ExecLimits::default());
    }

    #[test]
    fn a_timeout_replaces_only_the_time_limit() {
        let limits = ExecLimits::try_from(Some(30)).unwrap();

        assert_eq!(limits.timeout, Duration::from_secs(30));
        assert_eq!(limits.output_bytes, ExecLimits::default().output_bytes);
    }

    #[test]
    fn a_timeout_of_zero_or_over_a_day_is_refused() {
        assert!(ExecLimits::try_from(Some(0)).is_err());
        assert!(ExecLimits::try_from(Some(24 * 60 * 60 + 1)).is_err());
    }
}
