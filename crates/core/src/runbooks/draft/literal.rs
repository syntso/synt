use std::collections::BTreeSet;

use serde_json::Value;

use super::DraftStep;
use crate::runbooks::param::{ParamKind, ParamName, ParamSpec};

const TARGET_HINT: &str = "host";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Literal {
    text: String,
    hint: String,
    is_target: bool,
    steps: BTreeSet<usize>,
}

impl Literal {
    pub(super) fn repeated_in(steps: &[DraftStep]) -> Vec<Self> {
        let mut found = Vec::new();

        for (index, step) in steps.iter().enumerate() {
            Self::note_step(&mut found, index, step);
        }

        found.retain(Self::repeats);
        found
    }

    pub(super) fn text(&self) -> &str {
        &self.text
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

    pub(super) fn spec(&self, name: ParamName) -> ParamSpec {
        ParamSpec::literal(name, self.kind(), &self.text)
    }

    fn new(text: &str, hint: &str, is_target: bool, step: usize) -> Self {
        Self {
            text: text.to_owned(),
            hint: hint.to_owned(),
            is_target,
            steps: [step].into(),
        }
    }

    fn note_step(found: &mut Vec<Self>, index: usize, step: &DraftStep) {
        if let Some(target) = step.target() {
            Self::note(found, index, target, TARGET_HINT, true);
        }

        for (key, value) in step.params() {
            if let Some(text) = value.as_str() {
                Self::note(found, index, text, key, false);
            }
        }
    }

    fn note(found: &mut Vec<Self>, step: usize, text: &str, hint: &str, is_target: bool) {
        if text.is_empty() {
            return;
        }

        match found.iter_mut().find(|literal| literal.text == text) {
            Some(known) => known.seen_again(step, is_target),
            None => found.push(Self::new(text, hint, is_target, step)),
        }
    }

    fn seen_again(&mut self, step: usize, is_target: bool) {
        self.steps.insert(step);
        self.is_target |= is_target;
    }

    fn repeats(&self) -> bool {
        self.steps.len() > 1
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
