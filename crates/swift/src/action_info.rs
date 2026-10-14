use synt_proto::ActionDescription;

use crate::RiskLevel;

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ActionInfo {
    pub id: String,
    pub title: String,
    pub risk: RiskLevel,
    pub person_only: bool,
    pub takes_target: bool,
    pub params_schema: String,
}

impl From<ActionDescription> for ActionInfo {
    fn from(description: ActionDescription) -> Self {
        Self {
            id: description.id,
            title: description.title,
            risk: description.risk.into(),
            person_only: description.person_only,
            takes_target: description.takes_target,
            params_schema: description.params.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use synt_proto::ActionDescription;

    use super::ActionInfo;
    use crate::RiskLevel;

    #[test]
    fn descriptions_carry_their_schema_as_json() {
        let description = ActionDescription {
            id: "core.ping".to_owned(),
            title: "Ping".to_owned(),
            risk: synt_proto::RiskLevel::Read,
            person_only: false,
            takes_target: false,
            params: json!({ "type": "object" }),
        };

        let info = ActionInfo::from(description);

        assert_eq!(info.risk, RiskLevel::Read);
        assert_eq!(info.params_schema, r#"{"type":"object"}"#);
    }
}
