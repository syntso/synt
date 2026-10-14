use std::collections::HashMap;
use std::sync::Arc;

use synt_journal::ActionId;

use crate::{Action, ActionError, Catalog, Description, Handle};

#[derive(Clone, Default)]
pub struct Registry {
    actions: HashMap<ActionId, Handle>,
    catalogs: Vec<Arc<dyn Catalog>>,
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

    pub fn attach(&mut self, catalog: Arc<dyn Catalog>) {
        self.catalogs.push(catalog);
    }

    pub fn get(&self, id: &ActionId) -> Option<Handle> {
        if let Some(handle) = self.actions.get(id) {
            return Some(handle.clone());
        }

        self.catalogs.iter().find_map(|catalog| catalog.get(id))
    }

    pub fn ids(&self) -> impl Iterator<Item = &ActionId> {
        self.actions.keys()
    }

    pub fn describe(&self) -> Vec<Description> {
        let attached = self.catalogs.iter().flat_map(|catalog| catalog.describe());
        let mut descriptions: Vec<Description> = self
            .actions
            .values()
            .map(Handle::describe)
            .chain(attached)
            .collect();

        descriptions.sort_by(|left, right| left.id.as_str().cmp(right.id.as_str()));
        descriptions
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};
    use serde_json::{Value, json};
    use synt_journal::{ActionId, Actor, ResourceRef, Seq};

    use super::Registry;
    use crate::{Action, ActionError, Catalog, Context, Description, DynamicAction, Handle, Risk};

    struct Echo;

    #[derive(Deserialize, JsonSchema)]
    struct Shout {
        text: String,
    }

    #[derive(Serialize)]
    struct Echoed {
        text: String,
    }

    impl Action for Echo {
        const ID: &'static str = "test.echo";
        const TITLE: &'static str = "Echo text in capitals";
        const RISK: Risk = Risk::Read;

        type Params = Shout;
        type Output = Echoed;

        async fn run(&self, _context: &Context, params: Shout) -> Result<Echoed, ActionError> {
            Ok(Echoed {
                text: params.text.to_uppercase(),
            })
        }
    }

    struct Tail;

    #[derive(Deserialize, JsonSchema)]
    struct TailParams {
        follow: bool,
    }

    impl Action for Tail {
        const ID: &'static str = "test.tail";
        const TITLE: &'static str = "Read now, or keep reading";
        const RISK: Risk = Risk::Read;

        type Params = TailParams;
        type Output = ();

        fn finishes_later(&self, params: &TailParams) -> bool {
            params.follow
        }

        async fn run(&self, _context: &Context, _params: TailParams) -> Result<(), ActionError> {
            Ok(())
        }
    }

    struct Broken;

    impl Action for Broken {
        const ID: &'static str = "bad id\n";
        const TITLE: &'static str = "Never registered";
        const RISK: Risk = Risk::Write;

        type Params = ();
        type Output = ();

        async fn run(&self, _context: &Context, (): ()) -> Result<(), ActionError> {
            Ok(())
        }
    }

    struct Holler;

    impl DynamicAction for Holler {
        fn title(&self) -> &'static str {
            "Shout from a catalog"
        }

        fn risk(&self) -> Risk {
            Risk::Read
        }

        fn takes_target(&self) -> bool {
            false
        }

        fn params_schema(&self) -> Value {
            json!({ "type": "object" })
        }

        async fn run(
            &self,
            _context: &Context,
            _target: Option<ResourceRef>,
            _params: Value,
        ) -> Result<Value, ActionError> {
            Ok(json!("HEY"))
        }
    }

    struct Extra {
        handle: Handle,
    }

    impl Catalog for Extra {
        fn get(&self, id: &ActionId) -> Option<Handle> {
            (self.handle.id() == id).then(|| self.handle.clone())
        }

        fn describe(&self) -> Vec<Description> {
            vec![self.handle.describe()]
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
    fn a_call_can_say_it_finishes_later() {
        let mut registry = registry();
        registry.register(Tail).unwrap();
        let tail = registry.get(&ActionId::new("test.tail").unwrap()).unwrap();
        let echo = registry.get(&ActionId::new("test.echo").unwrap()).unwrap();

        let following = tail.prepare(None, json!({ "follow": true })).unwrap();
        let reading = tail.prepare(None, json!({ "follow": false })).unwrap();
        let echoing = echo.prepare(None, json!({ "text": "hi" })).unwrap();

        assert!(following.finishes_later());
        assert!(!reading.finishes_later());
        assert!(!echoing.finishes_later());
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

    #[tokio::test]
    async fn attached_catalogs_add_their_actions() {
        let mut registry = registry();
        let id = ActionId::new("extra.shout").unwrap();
        let handle = Handle::dynamic(id.clone(), Holler);
        registry.attach(Arc::new(Extra { handle }));

        let shout = registry.get(&id).unwrap();
        let output = shout
            .prepare(None, json!({}))
            .unwrap()
            .run(&context())
            .await;

        assert_eq!(output.map(|completed| completed.value), Ok(json!("HEY")));

        let ids: Vec<String> = registry
            .describe()
            .into_iter()
            .map(|description| description.id.as_str().to_owned())
            .collect();
        assert_eq!(ids, ["extra.shout", "test.echo"]);
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
