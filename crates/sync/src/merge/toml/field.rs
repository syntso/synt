use toml::{Table, Value};

#[derive(Debug, Default)]
pub(super) struct FieldMerge {
    conflicts: Vec<String>,
}

impl FieldMerge {
    pub(super) fn tables(
        &mut self,
        base: &Table,
        older: &Table,
        newer: &Table,
        prefix: &str,
    ) -> Table {
        let mut keys: Vec<&String> = older.keys().chain(newer.keys()).collect();
        keys.sort();
        keys.dedup();

        let mut merged = Table::new();

        for key in keys {
            let field = Self::field_name(prefix, key);
            let value = self.value(base.get(key), older.get(key), newer.get(key), &field);

            if let Some(value) = value {
                merged.insert(key.clone(), value);
            }
        }

        merged
    }

    pub(super) fn into_conflicts(self) -> Vec<String> {
        self.conflicts
    }

    fn value(
        &mut self,
        base: Option<&Value>,
        older: Option<&Value>,
        newer: Option<&Value>,
        field: &str,
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

            return Some(Value::Table(self.tables(base, older, newer, field)));
        }

        self.conflicts.push(field.to_owned());
        newer.or(older).cloned()
    }

    fn field_name(prefix: &str, key: &str) -> String {
        if prefix.is_empty() {
            return key.to_owned();
        }

        format!("{prefix}.{key}")
    }
}
