use serde_json::Value;

use crate::error::first_line;
use crate::objects::Fields;
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
        let documents: Vec<Value> = serde_saphyr::from_multiple(text).map_err(|error| {
            KubeError::invalid(format!("the manifest is not YAML: {}", first_line(&error)))
        })?;

        let mut objects = Vec::new();
        for document in documents.into_iter().filter(|document| !document.is_null()) {
            objects.extend(Self::items_of(document)?);
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

        objects.iter().try_for_each(Self::check)?;

        Ok(Self { objects })
    }

    pub fn objects(&self) -> &[Value] {
        &self.objects
    }

    pub fn refs(&self) -> Vec<ObjectRef> {
        self.objects.iter().map(ObjectRef::of).collect()
    }

    pub fn holds_secrets(&self) -> bool {
        self.objects
            .iter()
            .any(|object| object.text("kind") == Some(SECRET))
    }

    fn items_of(mut document: Value) -> Result<Vec<Value>, KubeError> {
        let is_list = document
            .text("kind")
            .is_some_and(|kind| kind.ends_with(LIST_SUFFIX));

        if !is_list {
            return Ok(vec![document]);
        }

        match document.get_mut("items").map(Value::take) {
            Some(Value::Array(items)) => Ok(items),
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
            if object.text(field).is_none_or(str::is_empty) {
                return Err(KubeError::invalid(format!(
                    "an object in the manifest has no {field}"
                )));
            }
        }

        if object.metadata_text("name").is_none_or(str::is_empty) {
            let kind = object.text("kind").unwrap_or_default();
            return Err(KubeError::invalid(format!(
                "a {kind} in the manifest has no metadata.name; generated names cannot be applied"
            )));
        }

        Ok(())
    }
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
    fn a_yaml_error_does_not_quote_the_manifest() {
        let text =
            "apiVersion: v1\nkind: Secret\nstringData:\n  password: hunter2\n  bad: [unclosed\n";

        let error = Manifest::parse(text).expect_err("broken").to_string();

        assert!(error.contains("line 5"), "{error}");
        assert!(!error.contains("hunter2"), "{error}");
    }

    #[test]
    fn text_that_is_not_yaml_is_refused() {
        assert!(Manifest::parse("kind: [unclosed").is_err());
        assert!(Manifest::parse("- just\n- a list\n").is_err());
    }
}
