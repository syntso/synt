use std::time::Duration;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const MOST_ATTEMPTS: u32 = 20;
const LONGEST_PAUSE: u64 = 3600;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Retry {
    pub attempts: u32,

    #[serde(default)]
    pub every_seconds: u64,
}

impl Retry {
    pub(crate) const ONCE: Self = Self {
        attempts: 1,
        every_seconds: 0,
    };

    pub(crate) fn check(&self) -> Result<(), String> {
        if !(1..=MOST_ATTEMPTS).contains(&self.attempts) {
            return Err(format!(
                "retry attempts must be 1 to {MOST_ATTEMPTS}, got {}",
                self.attempts
            ));
        }

        if self.every_seconds > LONGEST_PAUSE {
            return Err(format!(
                "retry every_seconds must be at most {LONGEST_PAUSE}, got {}",
                self.every_seconds
            ));
        }

        Ok(())
    }

    pub(crate) fn pause(&self) -> Duration {
        Duration::from_secs(self.every_seconds)
    }
}
