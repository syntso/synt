use synt_actions::Risk;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DescribedAction {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) risk: Risk,
    pub(crate) takes_target: bool,
    pub(crate) params_schema: String,
}
