use std::sync::Arc;

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use synt_actions::{Action, ActionError, Context, Risk, blocking};
use synt_journal::ResourceRef;
use tokio::runtime::Handle;

use super::rendered::Rendered;
use crate::Extensions;
use crate::builtin::CallingActor;
use crate::extensions::Guest;
use crate::runtime::GuestCall;
use crate::view::ViewCheck;

const LARGEST_TREE: usize = 1024 * 1024;

#[derive(Clone)]
pub(crate) struct ExtensionViewRender {
    extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RenderParams {
    extension: String,
    view: String,

    #[serde(default)]
    params: Value,
}

impl ExtensionViewRender {
    pub(crate) fn new(extensions: Extensions) -> Self {
        Self { extensions }
    }

    async fn render(
        guest: &Arc<Guest>,
        view: &str,
        call: GuestCall,
        context: Context,
    ) -> Result<String, ActionError> {
        let guest = Arc::clone(guest);
        let view = view.to_owned();

        let runtime = Handle::current();

        blocking(move || guest.render(&view, &call, context, runtime)).await
    }

    fn checked_tree(guest: &Guest, text: &str) -> Result<Value, ActionError> {
        if text.len() > LARGEST_TREE {
            return Err(ActionError::failed(format!(
                "the view is over {LARGEST_TREE} bytes"
            )));
        }

        let tree: Value = serde_json::from_str(text)
            .map_err(|error| ActionError::failed(format!("the view is not JSON: {error}")))?;
        let owns = |action: &str| guest.owns_action(action);

        ViewCheck::new(&owns)
            .tree(&tree)
            .map_err(|reason| ActionError::failed(format!("the view is malformed: {reason}")))?;

        Ok(tree)
    }
}

impl Action for ExtensionViewRender {
    const ID: &'static str = "extension.view.render";
    const TITLE: &'static str = "Render a native view of an extension as a view tree";
    const RISK: Risk = Risk::Read;

    type Params = RenderParams;
    type Output = Rendered;

    fn risk(&self, _target: Option<&ResourceRef>, params: &RenderParams) -> Risk {
        self.extensions
            .loaded(&params.extension)
            .map_or(Risk::Read, |loaded| loaded.ceiling())
    }

    fn destination(&self, target: Option<&ResourceRef>, _params: &RenderParams) -> Option<String> {
        target.map(|target| target.as_str().to_owned())
    }

    fn summary(&self, output: &Rendered) -> Value {
        output.summary()
    }

    async fn run(&self, context: &Context, params: RenderParams) -> Result<Rendered, ActionError> {
        context.refuse_extensions("reach the views, panels or actions of an extension")?;

        let guest = Arc::clone(self.extensions.loaded(&params.extension)?.guest());

        if !guest.manifest().contributes().has_view(&params.view) {
            return Err(ActionError::InvalidParams(format!(
                "{} contributes no view {}",
                params.extension, params.view
            )));
        }

        let call = GuestCall::new(
            String::new(),
            context.target().map(|target| target.as_str().to_owned()),
            params.params.to_string(),
        );
        let text = Self::render(&guest, &params.view, call, context.clone()).await?;
        let tree = Self::checked_tree(&guest, &text)?;

        Ok(Rendered::new(params.extension, params.view, tree))
    }
}
