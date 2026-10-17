use synt_extension_api::Error;
use synt_extension_api::files::{self, Access};
use synt_extension_api::serde_json::{Value, json};

use crate::params::Params;

pub fn chosen() -> Value {
    let picked: Vec<Value> = files::chosen()
        .into_iter()
        .map(|picked| {
            json!({
                "handle": picked.handle,
                "name": picked.name,
                "writable": picked.access == Access::ReadWrite,
            })
        })
        .collect();

    Value::Array(picked)
}

pub fn read(params: &Params) -> Result<Value, Error> {
    let bytes = files::read(params.text("handle")?)?;

    Ok(json!({ "text": String::from_utf8_lossy(&bytes) }))
}

pub fn write(params: &Params) -> Result<Value, Error> {
    files::write(params.text("handle")?, params.text("text")?.as_bytes())?;

    Ok(json!({ "written": params.text("handle")? }))
}
