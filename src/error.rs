use async_graphql::ErrorExtensions;
use thiserror::Error;
use tracing::{error, trace};

pub type Result<T> = async_graphql::Result<T>;

#[derive(Error, Debug)]
pub enum RtwalkError {
    #[error("Unauthenticated request")]
    UnauthenticatedRequest,
    #[error("Unauthorized bot owner")]
    UnauhorizedBotOwner,
    #[error("Internal server error")]
    InternalError(#[from] anyhow::Error),
    #[error("Internal server error")]
    ImpossibleError(&'static str, Option<anyhow::Error>),
    #[error("Username already exists")]
    UsernameAlreadyExists,
    #[error("Internal server error")]
    DatabaseError(#[from] surrealdb::Error),
    #[error("Internal server error")]
    RedisError(#[from] rustis::Error),
    #[error("Your verification code has expired. Register again.")]
    VerificationCodeExpired,
    #[error("Invalid verification code")]
    InvalidVerificationCode,
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Invalid password reset token")]
    InvalidPasswordResetToken,
    #[error("Max file upload size exceeded")]
    MaxUploadSizeExceeded,
}

impl ErrorExtensions for RtwalkError {
    fn extend(&self) -> async_graphql::Error {
        async_graphql::Error::new(format!("{}", self)).extend_with(|_err, e| match self {
            RtwalkError::UnauthenticatedRequest => {
                trace!("{}", self);
                e.set("tp", "UNAUTHENTICATED_REQUEST")
            }
            RtwalkError::InternalError(_)
            | RtwalkError::ImpossibleError(_, _)
            | RtwalkError::DatabaseError(_)
            | RtwalkError::RedisError(_) => {
                error!("{:?}", self);
                e.set("tp", "INTERNAL_ERROR")
            }
            RtwalkError::UsernameAlreadyExists => {
                trace!("{}", self);
                e.set("tp", "USERNAME_ALREADY_EXISTS")
            }
            RtwalkError::VerificationCodeExpired => {
                trace!("{}", self);
                e.set("tp", "VERIFICATION_CODE_EXPIRED")
            }
            RtwalkError::InvalidVerificationCode => {
                trace!("{}", self);
                e.set("tp", "INVALID_VERIFICATION_CODE")
            }
            RtwalkError::InvalidCredentials => {
                trace!("{}", self);
                e.set("tp", "INVALID_CREDENTIALS")
            }
            RtwalkError::UnauhorizedBotOwner => {
                trace!("{}", self);
                e.set("tp", "UNAUTHORIZED_BOT_OWNER")
            }
            RtwalkError::InvalidPasswordResetToken => {
                trace!("{}", self);
                e.set("tp", "INVALID_PASSWORD_RESET_TOKEN")
            }
            RtwalkError::MaxUploadSizeExceeded => {
                trace!("{}", self);
                e.set("tp", "MAX_UPLOAD_SIZE_EXCEEDED")
            }
        })
    }
}
