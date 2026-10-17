use serde::Serialize;
use serde::de::DeserializeOwned;

use super::Notification;

pub trait Notice: Serialize + DeserializeOwned {
    const METHOD: &'static str;

    fn to_notification(&self) -> Result<Notification, serde_json::Error> {
        let params = serde_json::to_value(self)?;

        Ok(Notification::new(Self::METHOD, params))
    }
}
