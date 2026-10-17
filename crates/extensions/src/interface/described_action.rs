use synt_actions::Risk;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DescribedAction {
    id: String,
    title: String,
    risk: Risk,
    takes_target: bool,
    params_schema: String,
}

impl DescribedAction {
    pub(crate) fn new(
        id: String,
        title: String,
        risk: Risk,
        takes_target: bool,
        params_schema: String,
    ) -> Self {
        Self {
            id,
            title,
            risk,
            takes_target,
            params_schema,
        }
    }

    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    pub(crate) fn risk(&self) -> Risk {
        self.risk
    }

    pub(crate) fn takes_target(&self) -> bool {
        self.takes_target
    }

    pub(crate) fn params_schema(&self) -> &str {
        &self.params_schema
    }
}
