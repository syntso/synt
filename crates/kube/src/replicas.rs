use schemars::JsonSchema;
use serde::Deserialize;

use crate::KubeError;

const LARGEST: u32 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(try_from = "u32")]
pub struct Replicas(u32);

impl Replicas {
    pub fn get(self) -> u32 {
        self.0
    }
}

impl TryFrom<u32> for Replicas {
    type Error = KubeError;

    fn try_from(count: u32) -> Result<Self, KubeError> {
        if count > LARGEST {
            return Err(KubeError::invalid(format!(
                "replicas must be at most {LARGEST}, got {count}"
            )));
        }

        Ok(Self(count))
    }
}

#[cfg(test)]
mod tests {
    use super::Replicas;

    #[test]
    fn up_to_ten_thousand_replicas_are_allowed() {
        assert_eq!(Replicas::try_from(10_000).map(Replicas::get), Ok(10_000));
    }

    #[test]
    fn more_are_refused() {
        assert!(Replicas::try_from(10_001).is_err());
    }
}
