use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum CliError {
    Usage(String),
    Io(std::io::Error),
    Json(serde_json::Error),
    Notify(notify::Error),
}

impl Display for CliError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(message) => f.write_str(message),
            Self::Io(err) => write!(f, "{err}"),
            Self::Json(err) => write!(f, "{err}"),
            Self::Notify(err) => write!(f, "{err}"),
        }
    }
}

impl From<std::io::Error> for CliError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for CliError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

impl From<notify::Error> for CliError {
    fn from(value: notify::Error) -> Self {
        Self::Notify(value)
    }
}

pub type Result<T> = std::result::Result<T, CliError>;
