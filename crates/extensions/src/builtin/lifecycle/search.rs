use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use super::SearchHit;
use crate::Extensions;
use crate::builtin::blocking::blocking;

#[derive(Clone)]
pub(crate) struct ExtensionSearch {
    extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, JsonSchema)]
pub(crate) struct SearchParams {
    #[serde(default)]
    pub query: Option<String>,
}

impl ExtensionSearch {
    pub(crate) fn new(extensions: Extensions) -> Self {
        Self { extensions }
    }
}

impl Action for ExtensionSearch {
    const ID: &'static str = "extension.search";
    const TITLE: &'static str = "Search the extension registry";
    const RISK: Risk = Risk::Read;

    type Params = SearchParams;
    type Output = Vec<SearchHit>;

    fn summary(&self, output: &Vec<SearchHit>) -> Value {
        json!({ "found": output.len() })
    }

    async fn run(
        &self,
        _context: &Context,
        params: SearchParams,
    ) -> Result<Vec<SearchHit>, ActionError> {
        let extensions = self.extensions.clone();

        blocking(move || extensions.search(params.query.as_deref())).await
    }
}
