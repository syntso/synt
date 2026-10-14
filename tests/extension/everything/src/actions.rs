use synt_extension_api::{ActionInfo, Risk};

const ACTIONS: [(&str, &str); 17] = [
    ("every.ping", "Call core.ping"),
    ("every.fetch", "Send an HTTP request"),
    ("every.settings", "Read the settings"),
    ("every.remember", "Keep a value in storage"),
    ("every.recall", "Read a value from storage"),
    ("every.arm", "Set a timer"),
    ("every.disarm", "Clear a timer"),
    ("every.heard", "Report what events, timers and the start left in storage"),
    ("every.write", "Write a note document"),
    ("every.read", "Read a note document"),
    ("every.list", "List the note documents"),
    ("every.remove", "Remove a note document"),
    ("every.files", "List the picked files"),
    ("every.read_file", "Read a picked file"),
    ("every.write_file", "Write a picked file"),
    ("every.patch", "Patch the overview view"),
    ("every.post", "Post a message to the editor panel"),
];

pub fn all() -> Vec<ActionInfo> {
    ACTIONS
        .iter()
        .map(|(id, title)| ActionInfo::new(*id, *title, Risk::Read).params_schema(r#"{"type":"object"}"#))
        .collect()
}
