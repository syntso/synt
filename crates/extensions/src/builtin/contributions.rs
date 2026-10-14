use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use super::contributed::Contributed;
use crate::Extensions;

#[derive(Clone)]
pub(crate) struct ExtensionContributions {
    extensions: Extensions,
}

impl ExtensionContributions {
    pub(crate) fn new(extensions: Extensions) -> Self {
        Self { extensions }
    }
}

impl Action for ExtensionContributions {
    const ID: &'static str = "extension.contributions";
    const TITLE: &'static str = "List what the loaded extensions add to the app";
    const RISK: Risk = Risk::Read;

    type Params = ();
    type Output = Vec<Contributed>;

    fn summary(&self, output: &Vec<Contributed>) -> Value {
        json!({ "extensions": output.len() })
    }

    async fn run(&self, _context: &Context, (): ()) -> Result<Vec<Contributed>, ActionError> {
        let mut contributed: Vec<Contributed> = self
            .extensions
            .all_loaded()
            .iter()
            .map(|loaded| loaded.guest.manifest())
            .filter(|manifest| !manifest.contributes.is_empty())
            .map(|manifest| Contributed {
                extension: manifest.id.clone(),
                name: manifest.name.clone(),
                contributes: manifest.contributes.clone(),
            })
            .collect();

        contributed.sort_by(|left, right| left.extension.cmp(&right.extension));
        Ok(contributed)
    }
}
