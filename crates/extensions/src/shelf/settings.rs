use rusqlite::OptionalExtension;
use serde_json::Value;

use super::Shelf;
use crate::ExtensionError;

impl Shelf {
    pub(crate) fn settings(&self, extension: &str) -> Result<Option<Value>, ExtensionError> {
        let text: Option<String> = self
            .lock()
            .query_row(
                "SELECT vals FROM extension_settings WHERE extension = ?1",
                [extension],
                |row| row.get(0),
            )
            .optional()?;

        let Some(text) = text else {
            return Ok(None);
        };

        serde_json::from_str(&text)
            .map(Some)
            .map_err(|error| ExtensionError::Refused(format!("stored settings: {error}")))
    }

    pub(crate) fn put_settings(
        &self,
        extension: &str,
        values: &Value,
    ) -> Result<(), ExtensionError> {
        self.lock().execute(
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
        let folder = tempfile::tempdir().unwrap();
        let shelf = Shelf::open(&folder.path().join("state.sqlite")).unwrap();

        shelf
            .put_settings("board", &json!({ "url": "https://x" }))
            .unwrap();

        assert_eq!(
            shelf.settings("board").unwrap(),
            Some(json!({ "url": "https://x" }))
        );
        assert_eq!(shelf.settings("other").unwrap(), None);

        shelf.forget("board").unwrap();
        assert_eq!(shelf.settings("board").unwrap(), None);
    }
}
