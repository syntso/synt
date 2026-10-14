use toml::{Table, Value};

use super::Merged;

pub(crate) const WHOLE_DOCUMENT: &str = "*";

pub(crate) fn merge_toml(base: Option<&[u8]>, older: &[u8], newer: &[u8]) -> Merged {
    let (Some(older_table), Some(newer_table)) = (parse(older), parse(newer)) else {
        return Merged::conflicted(Some(newer), WHOLE_DOCUMENT);
    };
    let base_table = base.and_then(parse).unwrap_or_default();

    let mut conflicts = Vec::new();
    let merged = merge_tables(&base_table, &older_table, &newer_table, "", &mut conflicts);

    Merged {
        body: Some(text(&merged, (older, &older_table), (newer, &newer_table))),
        conflicts,
    }
}

fn merge_tables(
    base: &Table,
    older: &Table,
    newer: &Table,
    prefix: &str,
    conflicts: &mut Vec<String>,
) -> Table {
    let mut keys: Vec<&String> = older.keys().chain(newer.keys()).collect();
    keys.sort();
    keys.dedup();

    let mut merged = Table::new();

    for key in keys {
        let field = field_name(prefix, key);
        let value = merge_value(
            base.get(key),
            older.get(key),
            newer.get(key),
            &field,
            conflicts,
        );

        if let Some(value) = value {
            merged.insert(key.clone(), value);
        }
    }

    merged
}

fn merge_value(
    base: Option<&Value>,
    older: Option<&Value>,
    newer: Option<&Value>,
    field: &str,
    conflicts: &mut Vec<String>,
) -> Option<Value> {
    if older == newer || base == newer {
        return older.cloned();
    }

    if base == older {
        return newer.cloned();
    }

    if let (Some(Value::Table(older)), Some(Value::Table(newer))) = (older, newer) {
        let empty = Table::new();
        let base = match base {
            Some(Value::Table(base)) => base,
            _ => &empty,
        };

        return Some(Value::Table(merge_tables(
            base, older, newer, field, conflicts,
        )));
    }

    conflicts.push(field.to_owned());
    newer.or(older).cloned()
}

fn field_name(prefix: &str, key: &str) -> String {
    if prefix.is_empty() {
        return key.to_owned();
    }

    format!("{prefix}.{key}")
}

fn parse(bytes: &[u8]) -> Option<Table> {
    let text = std::str::from_utf8(bytes).ok()?;

    text.parse::<Table>().ok()
}

fn text(merged: &Table, older: (&[u8], &Table), newer: (&[u8], &Table)) -> Vec<u8> {
    if merged == newer.1 {
        return newer.0.to_vec();
    }

    if merged == older.1 {
        return older.0.to_vec();
    }

    merged.to_string().into_bytes()
}

#[cfg(test)]
mod tests {
    use super::{WHOLE_DOCUMENT, merge_toml};

    fn merge(base: &str, older: &str, newer: &str) -> (String, Vec<String>) {
        let merged = merge_toml(Some(base.as_bytes()), older.as_bytes(), newer.as_bytes());

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
    fn without_a_common_base_equal_fields_agree_and_different_ones_conflict() {
        let merged = merge_toml(None, b"a = 1\nb = 2\n", b"a = 1\nb = 3\nc = 4\n");

        assert_eq!(
            table(&String::from_utf8(merged.body.unwrap()).unwrap()),
            table("a = 1\nb = 3\nc = 4\n")
        );
        assert_eq!(merged.conflicts, vec!["b"]);
    }

    #[test]
    fn a_side_that_is_not_toml_makes_the_newer_side_win_whole() {
        let merged = merge_toml(Some(b"a = 1"), b"a = [", b"a = 2");

        assert_eq!(merged.body.unwrap(), b"a = 2");
        assert_eq!(merged.conflicts, vec![WHOLE_DOCUMENT]);
    }
}
