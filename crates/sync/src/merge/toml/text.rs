use toml::Table;

use super::Layout;
use super::fields::FieldMerge;
use crate::merge::Merged;

pub(crate) const WHOLE_DOCUMENT: &str = "*";

pub(crate) struct TomlText<'a> {
    bytes: &'a [u8],
    table: Table,
}

impl<'a> TomlText<'a> {
    pub(crate) fn merge(base: Option<&[u8]>, older: &[u8], newer: &[u8]) -> Merged {
        let (Some(older_text), Some(newer_text)) = (TomlText::parse(older), TomlText::parse(newer))
        else {
            return Merged::conflicted(Some(newer), WHOLE_DOCUMENT);
        };
        let base_table = base
            .and_then(TomlText::parse)
            .map(|base| base.table)
            .unwrap_or_default();

        let mut fields = FieldMerge::default();
        let merged = fields.tables(&base_table, &older_text.table, &newer_text.table, "");

        Merged {
            body: Some(older_text.text(&merged, &newer_text)),
            conflicts: fields.into_conflicts(),
        }
    }

    fn parse(bytes: &'a [u8]) -> Option<Self> {
        let table = std::str::from_utf8(bytes).ok()?.parse().ok()?;

        Some(Self { bytes, table })
    }

    fn text(&self, merged: &Table, newer: &Self) -> Vec<u8> {
        if merged == &newer.table {
            return newer.bytes.to_vec();
        }

        if merged == &self.table {
            return self.bytes.to_vec();
        }

        self.rewritten(merged, newer)
            .unwrap_or_else(|| merged.to_string().into_bytes())
    }

    fn rewritten(&self, merged: &Table, newer: &Self) -> Option<Vec<u8>> {
        let older = Layout::parse(self.bytes)?;
        let newer = Layout::parse(newer.bytes)?;

        newer.reshaped(&older, merged)
    }
}

#[cfg(test)]
mod tests {
    use super::{TomlText, WHOLE_DOCUMENT};

    fn merge(base: &str, older: &str, newer: &str) -> (String, Vec<String>) {
        let merged = TomlText::merge(Some(base.as_bytes()), older.as_bytes(), newer.as_bytes());

        (
            String::from_utf8(merged.body.unwrap()).unwrap(),
            merged.conflicts,
        )
    }

    fn table(text: &str) -> toml::Table {
        text.parse().unwrap()
    }

    #[test]
    fn edits_to_different_fields_both_survive() {
        let base = "address = \"10.0.1.12\"\nuser = \"deploy\"\n";
        let older = "address = \"10.0.9.9\"\nuser = \"deploy\"\n";
        let newer = "address = \"10.0.1.12\"\nuser = \"ops\"\n";

        let (merged, conflicts) = merge(base, older, newer);

        assert_eq!(
            table(&merged),
            table("address = \"10.0.9.9\"\nuser = \"ops\"\n")
        );
        assert!(conflicts.is_empty());
    }

    #[test]
    fn the_newer_edit_of_the_same_field_wins_and_the_conflict_is_named() {
        let base = "port = 22\nuser = \"deploy\"\n";
        let older = "port = 2222\nuser = \"deploy\"\n";
        let newer = "port = 2200\nuser = \"deploy\"\n";

        let (merged, conflicts) = merge(base, older, newer);

        assert_eq!(table(&merged)["port"].as_integer(), Some(2200));
        assert_eq!(conflicts, vec!["port"]);
    }

    #[test]
    fn nested_tables_merge_field_by_field() {
        let base = "[params]\ncommand = \"uptime\"\ntimeout = 10\n";
        let older = "[params]\ncommand = \"uptime -p\"\ntimeout = 10\n";
        let newer = "[params]\ncommand = \"uptime\"\ntimeout = 30\n";

        let (merged, conflicts) = merge(base, older, newer);

        assert_eq!(
            table(&merged),
            table("[params]\ncommand = \"uptime -p\"\ntimeout = 30\n")
        );
        assert!(conflicts.is_empty());
    }

    #[test]
    fn a_field_removed_on_one_side_and_kept_on_the_other_is_removed() {
        let base = "via = \"bastion\"\nuser = \"deploy\"\n";
        let older = "user = \"deploy\"\n";
        let newer = "via = \"bastion\"\nuser = \"ops\"\n";

        let (merged, conflicts) = merge(base, older, newer);

        assert_eq!(table(&merged), table("user = \"ops\"\n"));
        assert!(conflicts.is_empty());
    }

    #[test]
    fn a_field_removed_on_one_side_and_changed_on_the_other_is_a_conflict() {
        let base = "via = \"bastion\"\n";
        let older = "via = \"jump\"\n";
        let newer = "";

        let (merged, conflicts) = merge(base, older, newer);

        assert_eq!(table(&merged), table("via = \"jump\"\n"));
        assert_eq!(conflicts, vec!["via"]);
    }

    #[test]
    fn arrays_are_one_field() {
        let base = "[[steps]]\naction = \"a\"\n";
        let older = "[[steps]]\naction = \"b\"\n";
        let newer = "[[steps]]\naction = \"c\"\n";

        let (merged, conflicts) = merge(base, older, newer);

        assert_eq!(table(&merged), table(newer));
        assert_eq!(conflicts, vec!["steps"]);
    }

    #[test]
    fn a_result_equal_to_one_side_keeps_its_exact_text() {
        let base = "a = 1\n";
        let older = "a = 1\n";
        let newer = "# a comment that a rewrite would lose\na = 2\n";

        let (merged, _) = merge(base, older, newer);

        assert_eq!(merged, newer);
    }

    #[test]
    fn comments_and_order_survive_a_merge_of_different_fields() {
        let base = "# the API host\nname = \"api\"\naddress = \"10.0.1.12\" # moved in May\nuser = \"deploy\"\n";
        let older = "# the API host\nname = \"api\"\naddress = \"10.0.9.9\" # moved in October\nuser = \"deploy\"\n";
        let newer = "# the API host\nname = \"api\"\naddress = \"10.0.1.12\" # moved in May\n# ops owns it now\nuser = \"ops\"\n";

        let (merged, conflicts) = merge(base, older, newer);

        assert_eq!(
            merged,
            "# the API host\nname = \"api\"\naddress = \"10.0.9.9\" # moved in October\n# ops owns it now\nuser = \"ops\"\n"
        );
        assert!(conflicts.is_empty());
    }

    #[test]
    fn comments_inside_nested_tables_survive_too() {
        let base = "[params]\n# what runs\ncommand = \"uptime\"\ntimeout = 10\n";
        let older = "[params]\n# what runs\ncommand = \"uptime -p\"\ntimeout = 10\n";
        let newer = "[params]\n# what runs\ncommand = \"uptime\"\n# seconds\ntimeout = 30\n";

        let (merged, conflicts) = merge(base, older, newer);

        assert_eq!(
            merged,
            "[params]\n# what runs\ncommand = \"uptime -p\"\n# seconds\ntimeout = 30\n"
        );
        assert!(conflicts.is_empty());
    }

    #[test]
    fn without_a_common_base_equal_fields_agree_and_different_ones_conflict() {
        let merged = TomlText::merge(None, b"a = 1\nb = 2\n", b"a = 1\nb = 3\nc = 4\n");

        assert_eq!(
            table(&String::from_utf8(merged.body.unwrap()).unwrap()),
            table("a = 1\nb = 3\nc = 4\n")
        );
        assert_eq!(merged.conflicts, vec!["b"]);
    }

    #[test]
    fn a_side_that_is_not_toml_makes_the_newer_side_win_whole() {
        let merged = TomlText::merge(Some(b"a = 1"), b"a = [", b"a = 2");

        assert_eq!(merged.body.unwrap(), b"a = 2");
        assert_eq!(merged.conflicts, vec![WHOLE_DOCUMENT]);
    }
}
