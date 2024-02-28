use async_graphql::ErrorExtensions;
use thiserror::Error;

pub type Result<T> = async_graphql::Result<T>;

#[derive(Error, Debug)]
pub enum RtwalkError {
    #[error("Unauthenticated request")]
    UnauthenticatedRequest,
    #[error("Internal server error")]
    InternalError,
    #[error("Username already exists")]
    UsernameAlreadyExists,
    #[error("Internal server error")]
    MongoError(#[from] mongodm::mongo::error::Error),
    #[error("Internal server error")]
    RedisError(#[from] rustis::Error),
    #[error("Your verification code has expired. Register again.")]
    VerificationCodeExpired,
    #[error("Invalid verification code")]
    InvalidVerificationCode,
}

impl ErrorExtensions for RtwalkError {
    fn extend(&self) -> async_graphql::Error {
        async_graphql::Error::new(format!("{}", self)).extend_with(|_err, e| match self {
            RtwalkError::UnauthenticatedRequest => e.set("tp", "UNAUTHENTICATED_REQUEST"),
            RtwalkError::InternalError
            | RtwalkError::MongoError(_)
            | RtwalkError::RedisError(_) => e.set("tp", "INTERNAL_ERROR"),
            RtwalkError::UsernameAlreadyExists => e.set("tp", "USERNAME_ALREADY_EXISTS"),
            RtwalkError::VerificationCodeExpired => e.set("tp", "VERIFICATION_CODE_EXPIRED"),
            RtwalkError::InvalidVerificationCode => e.set("tp", "INVALID_VERIFICATION_CODE"),
        })
    }
}
