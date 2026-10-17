use std::time::Duration;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::CoreError;

const MOST_ATTEMPTS: u32 = 20;
const LONGEST_PAUSE: u64 = 3600;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Retry {
    attempts: u32,

    #[serde(default)]
    every_seconds: u64,
}

impl Retry {
    pub(crate) const ONCE: Self = Self {
        attempts: 1,
        every_seconds: 0,
    };

    #[cfg(test)]
    pub(crate) fn new(attempts: u32, every_seconds: u64) -> Self {
        Self {
            attempts,
            every_seconds,
        }
    }

    pub(crate) fn check(&self) -> Result<(), CoreError> {
        if !(1..=MOST_ATTEMPTS).contains(&self.attempts) {
            return Err(CoreError::Attempts {
                attempts: self.attempts,
                most: MOST_ATTEMPTS,
            });
        }

        if self.every_seconds > LONGEST_PAUSE {
            return Err(CoreError::Pause {
                pause: self.every_seconds,
                longest: LONGEST_PAUSE,
            });
        }

        Ok(())
    }

    pub(crate) fn attempts(&self) -> u32 {
        self.attempts
    }

    pub(crate) fn every_seconds(&self) -> u64 {
        self.every_seconds
    }

    pub(crate) fn repeats(&self) -> bool {
        self.attempts > 1
    }

    pub(crate) fn pause(&self) -> Duration {
        Duration::from_secs(self.every_seconds)
    }
}
