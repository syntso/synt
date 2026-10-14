use std::sync::Arc;

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use super::rendered::Rendered;
use crate::Extensions;
use crate::builtin::CallingActor;
use crate::extension::{Guest, on_guest_thread};
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
    pub extension: String,
    pub view: String,

    #[serde(default)]
    pub params: Value,
}

impl ExtensionViewRender {
    pub(crate) fn new(extensions: Extensions) -> Self {
        Self { extensions }
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
            .map_or(Risk::Read, |loaded| loaded.ceiling)
    }

    fn destination(&self, target: Option<&ResourceRef>, _params: &RenderParams) -> Option<String> {
        target.map(|target| target.as_str().to_owned())
    }

    fn summary(&self, output: &Rendered) -> Value {
        json!({
            "extension": output.extension,
            "view": output.view,
            "bytes": output.tree.to_string().len(),
        })
    }

    async fn run(&self, context: &Context, params: RenderParams) -> Result<Rendered, ActionError> {
        context.refuse_extensions("reach the views, panels or actions of an extension")?;

        let loaded = self.extensions.loaded(&params.extension)?;
        let contributed = loaded
            .guest
            .manifest()
            .contributes
            .views()
            .contains(params.view.as_str());

        if !contributed {
            return Err(ActionError::InvalidParams(format!(
                "{} contributes no view {}",
                params.extension, params.view
            )));
        }

        let guest = Arc::clone(&loaded.guest);
        let view = params.view.clone();
        let call = GuestCall {
            action: String::new(),
            target: context.target().map(|target| target.as_str().to_owned()),
            params: params.params.to_string(),
        };
        let context = context.clone();

        let text =
            on_guest_thread(move |runtime| guest.render(&view, &call, context, runtime)).await?;
        let tree = checked_tree(&loaded.guest, &text)?;

        Ok(Rendered {
            extension: params.extension,
            view: params.view,
            tree,
        })
    }
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
