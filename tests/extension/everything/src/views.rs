use synt_extension_api::serde_json::{Value, json};
use synt_extension_api::view::{Node, Patch};
use synt_extension_api::{Error, storage, ui};

use crate::records::count;

pub fn render(view: &str, target: Option<&str>) -> Result<String, Error> {
    let tree = match view {
        "overview" => overview(target),
        "beat" => Node::text(format!("beats: {}", count("timer:beat"))),
        other => return Err(Error::new("unknown_view", other)),
    };

    Ok(tree.to_json())
}

pub fn patch() -> Result<Value, Error> {
    let patch = Patch::new().replace("/children/0/text", "patched");
    ui::patch_view("overview", &patch.to_json())?;

    Ok(json!({ "patched": "overview" }))
}

fn overview(target: Option<&str>) -> Node {
    let started = storage::get("started").is_some();

    Node::column([
        Node::title(format!("Everything on {}", target.unwrap_or("no host"))),
        Node::key_values([("started", json!(started))]),
        Node::button("Ping", "every.ping"),
    ])
}
