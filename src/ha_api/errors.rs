use std::fmt;

#[derive(Debug)]
pub enum Error {
    Request(reqwest::Error),
    Config(String),
    Refresh(),
}


impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        return Error::Request(error);
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Request(inner) => write!(f, "{}", inner),
            Error::Config(inner) => write!(f, "{}", inner),
            Error::Refresh() => write!(f, "Tried to refresh a long lived access token"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Request(inner) => Some(inner),
            Error::Config(_) => None,
            Error::Refresh() => None,
        }
    } 
}