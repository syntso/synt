use synt_extension_api::Error;
use synt_extension_api::documents::{self, DocumentInfo};
use synt_extension_api::serde_json::{Value, json};

use crate::params::Params;

pub fn write(params: &Params) -> Result<Value, Error> {
    let written = documents::write(
        params.text("project")?,
        params.text("path")?,
        params.text("text")?.as_bytes(),
        params.optional("base"),
    )?;

    Ok(json!({ "merged": written.merged, "info": info(&written.info) }))
}

pub fn read(params: &Params) -> Result<Value, Error> {
    let document = documents::read(params.text("project")?, params.text("path")?)?;

    Ok(json!({
        "text": String::from_utf8_lossy(&document.body),
        "info": info(&document.info),
    }))
}

pub fn list(params: &Params) -> Result<Value, Error> {
    let found = documents::listing(params.text("project")?)?;

    Ok(Value::Array(found.iter().map(info).collect()))
}

pub fn remove(params: &Params) -> Result<Value, Error> {
    documents::remove(params.text("project")?, params.text("path")?)?;

    Ok(json!({ "removed": params.text("path")? }))
}

fn info(info: &DocumentInfo) -> Value {
    json!({
        "project": info.project,
        "path": info.path,
        "kind": info.kind,
        "revision": info.revision,
        "size": info.size,
    })
}
