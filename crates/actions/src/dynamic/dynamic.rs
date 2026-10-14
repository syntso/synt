use std::sync::Arc;

use serde_json::{Map, Value};
use synt_journal::ResourceRef;

use super::DynamicAction;
use super::dynamic_bound::DynamicBound;
use crate::handle::Erased;
use crate::{ActionError, Prepared, Risk};

pub(crate) struct Dynamic<D> {
    action: Arc<D>,
}

impl<D: DynamicAction> Dynamic<D> {
    pub(crate) fn new(action: D) -> Self {
        Self {
            action: Arc::new(action),
        }
    }
}

impl<D: DynamicAction> Erased for Dynamic<D> {
    fn title(&self) -> &str {
        self.action.title()
    }

    fn declared_risk(&self) -> Risk {
        self.action.risk()
    }

    fn person_only(&self) -> bool {
        false
    }

    fn takes_target(&self) -> bool {
        self.action.takes_target()
    }

    fn params_schema(&self) -> Value {
        self.action.params_schema()
    }

    fn recorded_params(&self, params: &Value) -> Value {
        params.clone()
    }

    fn prepare(
        self: Arc<Self>,
        target: Option<&ResourceRef>,
        params: Value,
    ) -> Result<Prepared, ActionError> {
        if self.action.takes_target() && target.is_none() {
            return Err(ActionError::TargetRequired);
        }

        let params = object(params)?;
        let risk = self.action.risk();
        let destination = self.action.destination(target);

        let bound = DynamicBound::new(Arc::clone(&self.action), target.cloned(), params);

        Ok(Prepared::new(risk, destination, false, Box::new(bound)))
    }
}

fn object(params: Value) -> Result<Value, ActionError> {
    match params {
        Value::Null => Ok(Value::Object(Map::new())),
        Value::Object(fields) => Ok(Value::Object(fields)),
        _ => Err(ActionError::InvalidParams(
            "params must be a JSON object".to_owned(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};
    use synt_journal::{ActionId, Actor, ResourceRef, Seq};

    use crate::{ActionError, Context, DynamicAction, Handle, Risk};

    struct Greet;

    impl DynamicAction for Greet {
        fn title(&self) -> &'static str {
            "Greet a host"
        }

        fn risk(&self) -> Risk {
            Risk::Write
        }

        fn takes_target(&self) -> bool {
            true
        }

        fn params_schema(&self) -> Value {
            json!({ "type": "object" })
        }

        fn destination(&self, target: Option<&ResourceRef>) -> Option<String> {
            target.map(|target| target.as_str().to_owned())
        }

        async fn run(
            &self,
            _context: &Context,
            target: Option<ResourceRef>,
            params: Value,
        ) -> Result<Value, ActionError> {
            Ok(
                json!({ "target": target.map(|target| target.as_str().to_owned()), "params": params }),
            )
        }
    }

    fn handle() -> Handle {
        Handle::dynamic(ActionId::new("hello.greet").unwrap(), Greet)
    }

    fn target() -> ResourceRef {
        ResourceRef::new("web-1").unwrap()
    }

    #[tokio::test]
    async fn runs_with_target_and_params() {
        let prepared = handle()
            .prepare(Some(&target()), json!({ "name": "Ann" }))
            .unwrap();
        let context = Context::new(Actor::Person, Seq::FIRST, Some(target()));

        assert_eq!(prepared.risk(), Risk::Write);
        assert_eq!(prepared.destination(), Some("web-1"));

        let completed = prepared.run(&context).await.unwrap();

        assert_eq!(
            completed.value,
            json!({ "target": "web-1", "params": { "name": "Ann" } })
        );
    }

    #[test]
    fn describes_itself() {
        let description = handle().describe();

        assert_eq!(description.title, "Greet a host");
        assert!(description.takes_target);
        assert!(!description.person_only);
    }

    #[test]
    fn needs_a_target_when_it_takes_one() {
        assert!(matches!(
            handle().prepare(None, json!({})),
            Err(ActionError::TargetRequired)
        ));
    }

    #[test]
    fn refuses_params_that_are_not_an_object() {
        assert!(matches!(
            handle().prepare(Some(&target()), json!([1, 2])),
            Err(ActionError::InvalidParams(_))
        ));
    }
}
