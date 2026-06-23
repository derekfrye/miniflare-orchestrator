use std::fmt::{Display, Formatter};

pub const UNKNOWN_LEASE_PREFIX: &str = "unknown lease: ";
pub const HEALTH_CHECK_TIMED_OUT: &str = "health check timed out";
pub const HTTPS_REDIRECT_MESSAGE: &str =
    "health probe redirected to HTTPS; configure the lease protocol as https";

#[derive(Debug)]
pub enum LeaseError {
    Usage(String),
    Io(std::io::Error),
    Json(serde_json::Error),
    Utf8(std::string::FromUtf8Error),
    Base64(base64::DecodeError),
    NotFound(String),
    Conflict(String),
    Unavailable(String),
    Process(String),
}

impl LeaseError {
    #[must_use]
    pub fn usage(message: impl Into<String>) -> Self {
        Self::Usage(message.into())
    }

    #[must_use]
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::Unavailable(message.into())
    }

    #[must_use]
    pub fn process(message: impl Into<String>) -> Self {
        Self::Process(message.into())
    }

    #[must_use]
    pub fn not_found(id: impl Into<String>) -> Self {
        Self::NotFound(id.into())
    }

    #[must_use]
    pub fn unknown_lease(id: &str) -> Self {
        Self::NotFound(unknown_lease_message(id))
    }
}

#[must_use]
pub fn unknown_lease_message(id: &str) -> String {
    format!("{UNKNOWN_LEASE_PREFIX}{id}")
}

impl Display for LeaseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(message)
            | Self::NotFound(message)
            | Self::Conflict(message)
            | Self::Unavailable(message)
            | Self::Process(message) => f.write_str(message),
            Self::Io(error) => write!(f, "{error}"),
            Self::Json(error) => write!(f, "{error}"),
            Self::Utf8(error) => write!(f, "{error}"),
            Self::Base64(error) => write!(f, "{error}"),
        }
    }
}

impl From<std::io::Error> for LeaseError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for LeaseError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

impl From<std::string::FromUtf8Error> for LeaseError {
    fn from(value: std::string::FromUtf8Error) -> Self {
        Self::Utf8(value)
    }
}

impl From<base64::DecodeError> for LeaseError {
    fn from(value: base64::DecodeError) -> Self {
        Self::Base64(value)
    }
}
