use synt_extension_api::serde_json::{self, Value, json};
use synt_extension_api::{Assets, Error, ui};

use crate::params::Params;
use crate::reach;

const BUNDLE: Assets = Assets::new(&[
    ("index.html", include_bytes!("../panel/index.html")),
    ("app.js", include_bytes!("../panel/app.js")),
]);

pub fn answer(panel: &str, message: &str) -> Result<String, Error> {
    if panel != "editor" {
        return Err(Error::new("unknown_panel", panel));
    }

    let message: Value = serde_json::from_str(message)
        .map_err(|error| Error::new("invalid_message", error.to_string()))?;
    let pong = reach::ping()?;

    Ok(json!({ "echo": message, "pong": pong }).to_string())
}

pub fn asset(path: &str) -> Option<Vec<u8>> {
    BUNDLE.get(path)
}

pub fn post(params: &Params) -> Result<Value, Error> {
    let message = params.value("message");
    ui::post("editor", &message.to_string())?;

    Ok(json!({ "posted": message }))
}
