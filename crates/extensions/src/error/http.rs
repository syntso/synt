use thiserror::Error;

#[derive(Debug, Error)]
pub enum HttpError {
    #[error("{method} is not one of {allowed}")]
    Method { method: String, allowed: String },

    #[error("a request carries at most {0} headers")]
    TooManyHeaders(usize),

    #[error("the core sets the {0} header itself")]
    ReservedHeader(String),

    #[error("the {0} header is filled from a secret and cannot be set as well")]
    SecretClash(String),

    #[error("timeout_ms is 1 to {longest}, not {asked}")]
    Timeout { longest: u32, asked: u32 },

    #[error("the body is not base64: {0}")]
    BodyNotBase64(data_encoding::DecodeError),

    #[error("{0:?} is not a header name")]
    HeaderName(String),

    #[error("the {0} header has a value that cannot be sent")]
    HeaderValue(String),

    #[error("the {0} header cannot carry this secret")]
    SecretHeaderValue(String),

    #[error("{url:?} is not a URL: {source}")]
    NotUrl {
        url: String,
        source: ureq::http::uri::InvalidUri,
    },

    #[error("{0:?} must start with https:// or http://")]
    Scheme(String),

    #[error("{0:?} names no host this core may reach")]
    NoHost(String),

    #[error("{0:?} carries credentials; attach a secret instead")]
    Credentials(String),

    #[error("{0} is not listed in capabilities.http")]
    NotListed(String),

    #[error("the core keeps no secrets to attach")]
    NoSecrets,

    #[error("secrets go only over https or to this computer, not to http://{0}")]
    SecretOverPlainHttp(String),

    #[error("capabilities.secrets does not let secret://{secret} go to {destination}")]
    SecretNotAllowed { secret: String, destination: String },

    #[error("there is no secret secret://{0}")]
    NoSuchSecret(String),

    #[error("{0}")]
    SecretUnreadable(String),

    #[error("{0} is not text and cannot go in a header")]
    SecretNotText(String),

    #[error("{0} is empty or holds control characters and cannot go in a header")]
    SecretNotHeader(String),

    #[error("{url}: {source}")]
    Request {
        url: String,
        source: ureq::http::Error,
    },

    #[error("{url}: {source}")]
    Transport { url: String, source: ureq::Error },

    #[error("the response body: {0}")]
    ResponseBody(ureq::Error),

    #[error("the response body is not base64: {0}")]
    ResponseNotBase64(data_encoding::DecodeError),
}
