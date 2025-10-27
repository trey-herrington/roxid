use std::fmt;

pub type RpcResult<T> = Result<T, RpcError>;

#[derive(Debug)]
pub enum RpcError {
    ServiceError(String),
    InvalidRequest(String),
    InternalError(String),
}

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RpcError::ServiceError(msg) => write!(f, "Service error: {}", msg),
            RpcError::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            RpcError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for RpcError {}

impl From<pipeline_service::ServiceError> for RpcError {
    fn from(err: pipeline_service::ServiceError) -> Self {
        RpcError::ServiceError(err.to_string())
    }
}
