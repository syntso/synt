use serde_json::Value;

use crate::{KubeError, ObjectRef};

const LARGEST_COUNT: usize = 100;
const LIST_SUFFIX: &str = "List";
const SECRET: &str = "Secret";

#[derive(Debug, Clone, PartialEq)]
pub struct Manifest {
    objects: Vec<Value>,
}

impl Manifest {
    pub fn parse(text: &str) -> Result<Self, KubeError> {
        let documents: Vec<Value> = serde_saphyr::from_multiple(text)
            .map_err(|error| KubeError::invalid(format!("the manifest is not YAML: {error}")))?;

        let mut objects = Vec::new();
        for document in documents.into_iter().filter(|document| !document.is_null()) {
            objects.extend(expanded(document)?);
        }

        if objects.is_empty() {
            return Err(KubeError::invalid("the manifest holds no objects"));
        }
        if objects.len() > LARGEST_COUNT {
            return Err(KubeError::invalid(format!(
                "the manifest holds {} objects; apply at most {LARGEST_COUNT} at once",
                objects.len()
            )));
        }

        objects.iter().try_for_each(check)?;

        Ok(Self { objects })
    }

    pub fn objects(&self) -> &[Value] {
        &self.objects
    }

    pub fn refs(&self) -> Vec<ObjectRef> {
        self.objects.iter().map(object_ref).collect()
    }

    pub fn holds_secrets(&self) -> bool {
        self.objects
            .iter()
            .any(|object| text(object, "kind") == Some(SECRET))
    }
}

pub(crate) fn object_ref(object: &Value) -> ObjectRef {
    ObjectRef {
        kind: text(object, "kind").unwrap_or_default().to_owned(),
        name: metadata_text(object, "name").unwrap_or_default().to_owned(),
        namespace: metadata_text(object, "namespace").map(str::to_owned),
    }
}

pub(crate) fn text<'a>(object: &'a Value, field: &str) -> Option<&'a str> {
    object.get(field).and_then(Value::as_str)
}

pub(crate) fn metadata_text<'a>(object: &'a Value, field: &str) -> Option<&'a str> {
    object
        .get("metadata")
        .and_then(|metadata| text(metadata, field))
}

fn expanded(document: Value) -> Result<Vec<Value>, KubeError> {
    let is_list = text(&document, "kind").is_some_and(|kind| kind.ends_with(LIST_SUFFIX));

    if !is_list {
        return Ok(vec![document]);
    }

    match document.get("items") {
        Some(Value::Array(items)) => Ok(items.clone()),
        _ => Err(KubeError::invalid("a List in the manifest has no items")),
    }
}

fn check(object: &Value) -> Result<(), KubeError> {
    if !object.is_object() {
        return Err(KubeError::invalid(
            "every document of the manifest must be an object",
        ));
    }

    for field in ["apiVersion", "kind"] {
        if text(object, field).is_none_or(str::is_empty) {
            return Err(KubeError::invalid(format!(
                "an object in the manifest has no {field}"
            )));
        }
    }

    if metadata_text(object, "name").is_none_or(str::is_empty) {
        let kind = text(object, "kind").unwrap_or_default();
        return Err(KubeError::invalid(format!(
            "a {kind} in the manifest has no metadata.name; generated names cannot be applied"
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Manifest;

    const TWO: &str = "
apiVersion: v1
kind: ConfigMap
metadata:
  name: settings
  namespace: orders
data:
  mode: fast
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: api
";

    #[test]
    fn every_document_becomes_an_object() {
        let manifest = Manifest::parse(TWO).expect("two objects");
        let refs: Vec<String> = manifest.refs().iter().map(ToString::to_string).collect();

        assert_eq!(refs, ["ConfigMap orders/settings", "Deployment api"]);
        assert!(!manifest.holds_secrets());
    }

    #[test]
    fn json_is_yaml_too() {
        let manifest =
            Manifest::parse(r#"{"apiVersion":"v1","kind":"Secret","metadata":{"name":"key"}}"#)
                .expect("one secret");

        assert!(manifest.holds_secrets());
    }

    #[test]
    fn a_list_is_opened_into_its_items() {
        let list = r#"{"apiVersion":"v1","kind":"List","items":[
            {"apiVersion":"v1","kind":"Service","metadata":{"name":"web"}},
            {"apiVersion":"v1","kind":"Service","metadata":{"name":"db"}}]}"#;

        assert_eq!(Manifest::parse(list).expect("a list").objects().len(), 2);
    }

    #[test]
    fn empty_documents_are_skipped_but_an_empty_manifest_is_refused() {
        assert_eq!(
            Manifest::parse(&format!("---\n{TWO}---\n"))
                .expect("two")
                .objects()
                .len(),
            2
        );
        assert!(Manifest::parse("---\n").is_err());
    }

    #[test]
    fn an_object_without_a_name_is_refused_with_a_reason() {
        let error = Manifest::parse("apiVersion: v1\nkind: Pod\nmetadata:\n  generateName: web-\n")
            .expect_err("no name");

        assert!(error.to_string().contains("metadata.name"));
    }

    #[test]
    fn text_that_is_not_yaml_is_refused() {
        assert!(Manifest::parse("kind: [unclosed").is_err());
        assert!(Manifest::parse("- just\n- a list\n").is_err());
    }
}
