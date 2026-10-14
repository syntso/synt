use std::collections::BTreeSet;

use serde_json::Value;

use crate::runbooks::param::{ParamKind, ParamSpec};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Literal {
    pub(super) text: String,
    hint: String,
    is_target: bool,
    steps: BTreeSet<usize>,
}

impl Literal {
    pub(super) fn new(text: &str, hint: &str, is_target: bool, step: usize) -> Self {
        Self {
            text: text.to_owned(),
            hint: hint.to_owned(),
            is_target,
            steps: [step].into(),
        }
    }

    pub(super) fn seen_again(&mut self, step: usize, is_target: bool) {
        self.steps.insert(step);
        self.is_target |= is_target;
    }

    pub(super) fn repeats(&self) -> bool {
        self.steps.len() > 1
    }

    pub(super) fn base_name(&self) -> String {
        let cleaned: String = self
            .hint
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() {
                    character
                } else {
                    '_'
                }
            })
            .collect();

        match cleaned.chars().next() {
            Some(first) if first.is_ascii_alphabetic() || first == '_' => cleaned,
            _ => format!("_{cleaned}"),
        }
    }

    pub(super) fn spec(&self, name: &str) -> ParamSpec {
        ParamSpec::literal(name.to_owned(), self.kind(), &self.text)
    }

    fn kind(&self) -> ParamKind {
        let as_target = self.is_target
            && ParamKind::Target
                .accept(&Value::from(self.text.as_str()))
                .is_ok();

        if as_target {
            return ParamKind::Target;
        }

        ParamKind::of_literal(&self.text)
    }
}
