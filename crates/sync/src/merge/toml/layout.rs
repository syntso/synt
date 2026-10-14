use toml::{Table, Value};
use toml_edit::{DocumentMut, Item, TableLike};

pub(super) struct Layout {
    document: DocumentMut,
}

impl Layout {
    pub(super) fn parse(bytes: &[u8]) -> Option<Self> {
        let document = std::str::from_utf8(bytes).ok()?.parse().ok()?;

        Some(Self { document })
    }

    pub(super) fn reshaped(mut self, older: &Self, merged: &Table) -> Option<Vec<u8>> {
        Self::apply(
            self.document.as_table_mut(),
            Some(older.document.as_table()),
            merged,
        );

        let text = self.document.to_string();
        let faithful = text.parse::<Table>().ok().as_ref() == Some(merged);

        faithful.then(|| text.into_bytes())
    }

    fn apply(target: &mut dyn TableLike, older: Option<&dyn TableLike>, merged: &Table) {
        Self::remove_missing(target, merged);

        for (key, wanted) in merged {
            let older_item = older.and_then(|older| older.get(key));

            Self::apply_field(target, older_item, key, wanted);
        }
    }

    fn remove_missing(target: &mut dyn TableLike, merged: &Table) {
        let stale: Vec<String> = target
            .iter()
            .map(|(key, _)| key.to_owned())
            .filter(|key| !merged.contains_key(key))
            .collect();

        for key in stale {
            target.remove(&key);
        }
    }

    fn apply_field(target: &mut dyn TableLike, older: Option<&Item>, key: &str, wanted: &Value) {
        let current = target.get(key).and_then(|item| item_value(key, item));
        if current.as_ref() == Some(wanted) {
            return;
        }

        if let (Some(inner), Value::Table(wanted)) = (
            target.get_mut(key).and_then(Item::as_table_like_mut),
            wanted,
        ) {
            Self::apply(inner, older.and_then(Item::as_table_like), wanted);
            return;
        }

        let replacement = older
            .filter(|item| item_value(key, item).as_ref() == Some(wanted))
            .cloned()
            .or_else(|| fresh_item(key, wanted));

        match (target.get_mut(key), replacement) {
            (Some(item), Some(replacement)) => *item = replacement,
            (None, Some(replacement)) => {
                target.insert(key, replacement);
            }
            (_, None) => {}
        }
    }
}

fn item_value(key: &str, item: &Item) -> Option<Value> {
    let mut alone = DocumentMut::new();
    alone.insert(key, item.clone());

    alone.to_string().parse::<Table>().ok()?.remove(key)
}

fn fresh_item(key: &str, value: &Value) -> Option<Item> {
    let mut alone = Table::new();
    alone.insert(key.to_owned(), value.clone());

    Layout::parse(alone.to_string().as_bytes())?
        .document
        .remove(key)
}
