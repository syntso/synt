use serde_json::json;
use synt_actions::{Catalog, Description, Handle, Risk};
use synt_journal::ActionId;

const ACTIONS: [(&str, Risk); 3] = [
    ("core.ping", Risk::Read),
    ("ssh.exec", Risk::Write),
    ("sftp.write", Risk::Write),
];

pub struct CoreStub;

impl Catalog for CoreStub {
    fn get(&self, _id: &ActionId) -> Option<Handle> {
        None
    }

    fn describe(&self) -> Vec<Description> {
        ACTIONS
            .iter()
            .map(|(id, risk)| Description {
                id: ActionId::new(*id).unwrap(),
                title: format!("Stand in for {id}"),
                risk: *risk,
                person_only: false,
                takes_target: true,
                params: json!({ "type": "object" }),
            })
            .collect()
    }
}
