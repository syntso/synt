use super::DraftStep;
use super::literal::Literal;

const TARGET_HINT: &str = "host";

#[derive(Debug, Default)]
pub(super) struct Literals {
    found: Vec<Literal>,
}

impl Literals {
    pub(super) fn repeated_in(steps: &[DraftStep]) -> Vec<Literal> {
        let mut literals = Self::default();

        for (index, step) in steps.iter().enumerate() {
            literals.note_step(index, step);
        }

        literals.found.retain(Literal::repeats);
        literals.found
    }

    fn note_step(&mut self, index: usize, step: &DraftStep) {
        if let Some(target) = &step.target {
            self.note(index, target, TARGET_HINT, true);
        }

        for (key, value) in &step.params {
            if let Some(text) = value.as_str() {
                self.note(index, text, key, false);
            }
        }
    }

    fn note(&mut self, step: usize, text: &str, hint: &str, is_target: bool) {
        if text.is_empty() {
            return;
        }

        match self.found.iter_mut().find(|literal| literal.text == text) {
            Some(known) => known.seen_again(step, is_target),
            None => self.found.push(Literal::new(text, hint, is_target, step)),
        }
    }
}
