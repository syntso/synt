use serde_json::{Map, Value};

const LONGEST_TITLE: usize = 80;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DraftStep {
    pub(crate) title: String,
    pub(crate) action: String,
    pub(crate) target: Option<String>,
    pub(crate) params: Map<String, Value>,
}

impl DraftStep {
    pub(crate) fn title_from(text: &str) -> String {
        let line = text.lines().next().unwrap_or_default().trim();
        let line: String = line
            .chars()
            .filter(|character| !character.is_control())
            .collect();

        if line.chars().count() <= LONGEST_TITLE {
            return line;
        }

        let shortened: String = line.chars().take(LONGEST_TITLE - 1).collect();
        format!("{shortened}…")
    }
}

#[cfg(test)]
mod tests {
    use super::DraftStep;

    #[test]
    fn a_title_is_the_first_line_kept_short() {
        assert_eq!(DraftStep::title_from("uptime\nrm x"), "uptime");
        assert_eq!(DraftStep::title_from(&"a".repeat(100)).chars().count(), 80);
        assert!(DraftStep::title_from(&"a".repeat(100)).ends_with('…'));
    }
}
