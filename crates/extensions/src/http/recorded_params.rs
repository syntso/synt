use data_encoding::BASE64;
use serde_json::{Value, json};

const HIDDEN: &str = "?";

pub(crate) struct RecordedParams<'v> {
    params: &'v Value,
}

impl<'v> RecordedParams<'v> {
    pub(crate) fn new(params: &'v Value) -> Self {
        Self { params }
    }

    pub(crate) fn to_value(&self) -> Value {
        let headers: Vec<&Value> = self
            .list("headers")
            .iter()
            .filter_map(|header| header.get("name"))
            .collect();
        let secrets: Vec<Value> = self
            .list("secrets")
            .iter()
            .map(|secret| json!({ "name": secret.get("name"), "secret": secret.get("secret") }))
            .collect();
        let body_bytes = self.text("body").map(Self::decoded_length);

        json!({
            "method": self.params.get("method"),
            "url": self.text("url").map(Self::hidden_url),
            "headers": headers,
            "secrets": secrets,
            "body_bytes": body_bytes,
            "timeout_ms": self.params.get("timeout_ms"),
        })
    }

    fn list(&self, field: &str) -> &'v [Value] {
        self.params
            .get(field)
            .and_then(Value::as_array)
            .map_or(&[], Vec::as_slice)
    }

    fn text(&self, field: &str) -> Option<&'v str> {
        self.params.get(field).and_then(Value::as_str)
    }

    fn decoded_length(base64: &str) -> usize {
        BASE64
            .decode(base64.as_bytes())
            .map_or(0, |bytes| bytes.len())
    }

    fn hidden_url(url: &str) -> String {
        let without_fragment = url.split('#').next().unwrap_or_default();
        let (place, query) = match without_fragment.split_once('?') {
            Some((place, query)) => (place, Some(query)),
            None => (without_fragment, None),
        };

        let place = Self::without_user(place);

        let Some(query) = query else {
            return place;
        };

        let hidden: Vec<String> = query
            .split('&')
            .map(|pair| match pair.split_once('=') {
                Some((name, _)) => format!("{name}={HIDDEN}"),
                None => HIDDEN.to_owned(),
            })
            .collect();

        format!("{place}?{}", hidden.join("&"))
    }

    fn without_user(place: &str) -> String {
        let Some((scheme, rest)) = place.split_once("://") else {
            return place.to_owned();
        };

        let (authority, path) = rest.split_at(rest.find('/').unwrap_or(rest.len()));

        match authority.rsplit_once('@') {
            Some((_, host)) => format!("{scheme}://{HIDDEN}@{host}{path}"),
            None => place.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RecordedParams;

    #[test]
    fn query_values_are_hidden_and_their_names_kept() {
        assert_eq!(
            RecordedParams::hidden_url(
                "https://api.example.com/v1/search?q=ann%40example.com&api_key=abc123&flag"
            ),
            "https://api.example.com/v1/search?q=?&api_key=?&?"
        );
    }

    #[test]
    fn a_plain_url_is_kept_and_a_fragment_dropped() {
        assert_eq!(
            RecordedParams::hidden_url("https://api.example.com:8443/v1/health"),
            "https://api.example.com:8443/v1/health"
        );
        assert_eq!(
            RecordedParams::hidden_url("https://api.example.com/page#token=abc"),
            "https://api.example.com/page"
        );
    }

    #[test]
    fn credentials_in_the_address_are_hidden() {
        assert_eq!(
            RecordedParams::hidden_url("https://ann:hunter2@api.example.com/v1?x=1"),
            "https://?@api.example.com/v1?x=?"
        );
    }
}
