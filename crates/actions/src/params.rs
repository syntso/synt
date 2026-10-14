use std::any::TypeId;

use schemars::JsonSchema;
use schemars::generate::SchemaSettings;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};

use crate::ActionError;

pub(crate) fn schema<P: JsonSchema + 'static>() -> Value {
    if takes_none::<P>() {
        return json!({ "type": "object", "properties": {} });
    }

    let generator = SchemaSettings::draft2020_12()
        .with(|settings| {
            settings.inline_subschemas = true;
            settings.meta_schema = None;
        })
        .into_generator();

    let mut schema = generator.into_root_schema_for::<P>().to_value();

    if let Some(root) = schema.as_object_mut() {
        root.remove("title");
    }

    schema
}

pub(crate) fn parse<P: DeserializeOwned + 'static>(params: Value) -> Result<P, ActionError> {
    serde_json::from_value(normalized::<P>(params))
        .map_err(|error| ActionError::InvalidParams(error.to_string()))
}

fn normalized<P: 'static>(params: Value) -> Value {
    let takes_none = takes_none::<P>();

    match params {
        Value::Object(fields) if takes_none && fields.is_empty() => Value::Null,
        Value::Null if !takes_none => Value::Object(Map::new()),
        params => params,
    }
}

fn takes_none<P: 'static>() -> bool {
    TypeId::of::<P>() == TypeId::of::<()>()
}
