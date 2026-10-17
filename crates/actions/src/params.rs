use std::any::TypeId;

use schemars::JsonSchema;
use schemars::generate::SchemaSettings;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};

use crate::ActionError;

pub(crate) trait Params: DeserializeOwned + JsonSchema + Sized + 'static {
    fn schema() -> Value {
        if Self::takes_none() {
            return json!({ "type": "object", "properties": {} });
        }

        let generator = SchemaSettings::draft2020_12()
            .with(|settings| {
                settings.inline_subschemas = true;
                settings.meta_schema = None;
            })
            .into_generator();

        let mut schema = generator.into_root_schema_for::<Self>().to_value();

        if let Some(root) = schema.as_object_mut() {
            root.remove("title");
        }

        schema
    }

    fn parse(params: Value) -> Result<Self, ActionError> {
        let params = Self::normalized(params);

        serde_json::from_value(params)
            .map_err(|error| ActionError::InvalidParams(error.to_string()))
    }

    fn normalized(params: Value) -> Value {
        let takes_none = Self::takes_none();

        match params {
            Value::Object(fields) if takes_none && fields.is_empty() => Value::Null,
            Value::Null if !takes_none => Value::Object(Map::new()),
            params => params,
        }
    }

    fn takes_none() -> bool {
        TypeId::of::<Self>() == TypeId::of::<()>()
    }
}

impl<P: DeserializeOwned + JsonSchema + 'static> Params for P {}
