use serde_json::json;
use synt_actions::{ActionDescription, Catalog, RegisteredAction, Risk};
use synt_journal::ActionId;

const ACTIONS: [(&str, Risk); 3] = [
    ("core.ping", Risk::Read),
    ("ssh.exec", Risk::Write),
    ("sftp.write", Risk::Write),
];

pub struct CoreStub;

impl Catalog for CoreStub {
    fn get(&self, _id: &ActionId) -> Option<RegisteredAction> {
        None
    }

    fn describe(&self) -> Vec<ActionDescription> {
        ACTIONS
            .iter()
            .filter_map(|(id, risk)| {
                let action = ActionId::new(*id).ok()?;

                Some(ActionDescription {
                    id: action,
                    title: format!("Stand in for {id}"),
                    risk: *risk,
                    person_only: false,
                    takes_target: true,
                    params: json!({ "type": "object" }),
                })
            })
            .collect()
    }
}
