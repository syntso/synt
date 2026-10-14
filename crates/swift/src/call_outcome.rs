use synt_proto::ActionOutcome;

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct CallOutcome {
    pub request: u64,
    pub value: String,
}

impl From<ActionOutcome> for CallOutcome {
    fn from(outcome: ActionOutcome) -> Self {
        Self {
            request: outcome.request.0,
            value: outcome.value.to_string(),
        }
    }
}
