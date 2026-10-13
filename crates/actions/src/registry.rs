use std::collections::HashMap;

use synt_journal::ActionId;

use crate::{Action, ActionError, Handle};

#[derive(Clone, Default)]
pub struct Registry {
    actions: HashMap<ActionId, Handle>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<A: Action>(&mut self, action: A) -> Result<(), ActionError> {
        let id =
            ActionId::new(A::ID).map_err(|reason| ActionError::InvalidId { id: A::ID, reason })?;

        if self.actions.contains_key(&id) {
            return Err(ActionError::Duplicate(A::ID));
        }

        self.actions.insert(id.clone(), Handle::new(id, action));
        Ok(())
    }

    pub fn get(&self, id: &ActionId) -> Option<&Handle> {
        self.actions.get(id)
    }

    pub fn ids(&self) -> impl Iterator<Item = &ActionId> {
        self.actions.keys()
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_json::json;
    use synt_journal::{ActionId, Actor, Seq};

    use super::Registry;
    use crate::{Action, ActionError, Context, Risk};

    struct Echo;

    #[derive(Deserialize)]
    struct Shout {
        text: String,
    }

    #[derive(Serialize)]
    struct Echoed {
        text: String,
    }

    impl Action for Echo {
        const ID: &'static str = "test.echo";
        const RISK: Risk = Risk::Read;

        type Params = Shout;
        type Output = Echoed;

        async fn run(&self, _context: &Context, params: Shout) -> Result<Echoed, ActionError> {
            Ok(Echoed {
                text: params.text.to_uppercase(),
            })
        }
    }

    struct Broken;

    impl Action for Broken {
        const ID: &'static str = "bad id\n";
        const RISK: Risk = Risk::Write;

        type Params = ();
        type Output = ();

        async fn run(&self, _context: &Context, (): ()) -> Result<(), ActionError> {
            Ok(())
        }
    }

    fn context() -> Context {
        Context::new(Actor::Person, Seq::FIRST, None)
    }

    fn registry() -> Registry {
        let mut registry = Registry::new();
        registry.register(Echo).unwrap();
        registry
    }

    #[tokio::test]
    async fn calls_actions_through_json() {
        let registry = registry();
        let echo = registry.get(&ActionId::new("test.echo").unwrap()).unwrap();

        let prepared = echo.prepare(None, json!({ "text": "hi" })).unwrap();
        let output = prepared.run(&context()).await;

        assert_eq!(
            output.map(|completed| completed.value),
            Ok(json!({ "text": "HI" }))
        );
        assert_eq!(echo.risk(), Risk::Read);
    }

    #[test]
    fn malformed_params_never_reach_the_action() {
        let registry = registry();
        let echo = registry.get(&ActionId::new("test.echo").unwrap()).unwrap();

        let prepared = echo.prepare(None, json!({ "wrong": 1 }));

        assert!(matches!(prepared, Err(ActionError::InvalidParams(_))));
    }

    #[test]
    fn ids_are_registered_once() {
        let mut registry = registry();
        assert_eq!(
            registry.register(Echo),
            Err(ActionError::Duplicate("test.echo"))
        );
    }

    #[test]
    fn invalid_ids_are_refused() {
        let mut registry = Registry::new();
        assert!(matches!(
            registry.register(Broken),
            Err(ActionError::InvalidId { .. })
        ));
    }

    #[test]
    fn unknown_ids_are_absent() {
        assert!(
            registry()
                .get(&ActionId::new("test.missing").unwrap())
                .is_none()
        );
    }
}
