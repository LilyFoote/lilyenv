#[derive(Debug)]
pub enum Error {
    Request(reqwest::Error),
    Octocrab(octocrab::Error),
    Scraper(String),
    Url(url::ParseError),
    Fs(std::io::Error),
    VersionNotFound(String),
    InvalidVersion(String),
    ParseAsset(String),
    Platform(String),
    EnvVar(std::env::VarError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Request(err) => write!(f, "{err}"),
            Self::Octocrab(err) => write!(f, "{err}"),
            Self::Fs(err) => write!(f, "{err}"),
            Self::Url(err) => write!(f, "{err}"),
            Self::VersionNotFound(version) => write!(f, "Could not find {version} to download."),
            Self::InvalidVersion(version) => write!(f, "{version} is not a valid Python version"),
            Self::ParseAsset(asset) => {
                write!(f, "Could not parse version and release_tag from {asset}.")
            }
            Self::Scraper(error) => write!(f, "{error}"),
            Self::Platform(platform) => write!(f, "{platform} is not supported."),
            Self::EnvVar(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Self::Request(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Fs(err)
    }
}

impl From<octocrab::Error> for Error {
    fn from(err: octocrab::Error) -> Self {
        Self::Octocrab(err)
    }
}

impl From<url::ParseError> for Error {
    fn from(err: url::ParseError) -> Self {
        Self::Url(err)
    }
}

impl From<std::env::VarError> for Error {
    fn from(err: std::env::VarError) -> Self {
        Self::EnvVar(err)
    }
}
