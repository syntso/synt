use rusqlite::OptionalExtension;
use serde_json::Value;

use super::Sqlite;
use crate::ExtensionError;

#[derive(Debug, Clone)]
pub(crate) struct StoredSettings {
    sqlite: Sqlite,
}

impl StoredSettings {
    pub(super) fn new(sqlite: Sqlite) -> Self {
        Self { sqlite }
    }

    pub(crate) fn get(&self, extension: &str) -> Result<Option<Value>, ExtensionError> {
        let text: Option<String> = self
            .sqlite
            .lock()
            .query_row(
                "SELECT vals FROM extension_settings WHERE extension = ?1",
                [extension],
                |row| row.get("vals"),
            )
            .optional()?;

        let Some(text) = text else {
            return Ok(None);
        };

        let values = serde_json::from_str(&text).map_err(ExtensionError::StoredSettings)?;

        Ok(Some(values))
    }

    pub(crate) fn put(&self, extension: &str, values: &Value) -> Result<(), ExtensionError> {
        self.sqlite.lock().execute(
            "INSERT OR REPLACE INTO extension_settings (extension, vals) VALUES (?1, ?2)",
            (extension, values.to_string()),
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::shelf::Shelf;

    #[test]
    fn keeps_settings_per_extension_until_it_is_forgotten() {
        let (_folder, shelf) = Shelf::scratch();
        let settings = shelf.settings();

        settings
            .put("board", &json!({ "url": "https://x" }))
            .unwrap();

        assert_eq!(
            settings.get("board").unwrap(),
            Some(json!({ "url": "https://x" }))
        );
        assert_eq!(settings.get("other").unwrap(), None);

        shelf.inventory().forget("board").unwrap();
        assert_eq!(settings.get("board").unwrap(), None);
    }
}
