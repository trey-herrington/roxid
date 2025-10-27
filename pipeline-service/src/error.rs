use std::fmt;

pub type ServiceResult<T> = Result<T, ServiceError>;

#[derive(Debug)]
pub enum ServiceError {
    NotFound(String),
    InvalidInput(String),
    Internal(String),
    IoError(std::io::Error),
    YamlError(serde_yaml::Error),
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceError::NotFound(msg) => write!(f, "Not found: {}", msg),
            ServiceError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            ServiceError::Internal(msg) => write!(f, "Internal error: {}", msg),
            ServiceError::IoError(e) => write!(f, "IO error: {}", e),
            ServiceError::YamlError(e) => write!(f, "YAML error: {}", e),
        }
    }
}

impl std::error::Error for ServiceError {}

impl From<std::io::Error> for ServiceError {
    fn from(err: std::io::Error) -> Self {
        ServiceError::IoError(err)
    }
}

impl From<serde_yaml::Error> for ServiceError {
    fn from(err: serde_yaml::Error) -> Self {
        ServiceError::YamlError(err)
    }
}
