use std::fmt::{self, Display, Formatter};
use std::time::Duration;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    JsonSchema,
)]
#[serde(transparent)]
pub struct Millis(u64);

impl Millis {
    pub const fn new(millis: u64) -> Self {
        Self(millis)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

impl From<Duration> for Millis {
    fn from(duration: Duration) -> Self {
        Self(u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
    }
}

impl Display for Millis {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} ms", self.0)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::Millis;

    #[test]
    fn a_duration_becomes_whole_milliseconds_and_travels_as_a_number() {
        let millis = Millis::from(Duration::from_micros(15_900));

        assert_eq!(millis, Millis::new(15));
        assert_eq!(serde_json::to_value(millis).unwrap(), serde_json::json!(15));
        assert_eq!(Millis::from(Duration::MAX), Millis::new(u64::MAX));
    }
}
