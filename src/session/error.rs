use thiserror::Error;

#[derive(Error, Debug)]
pub enum SessionError {
    #[error("session receieved invalid argument")]
    InvalidArgs(#[from] std::ffi::NulError),
    #[error("session failed to connect")]
    ConnectionFailure,
    #[error("session failed to initialize")]
    InitializationFailure,
    #[error("session failed to subscribe on topic: {0}")]
    SubscriptionFailure(String),
    #[error("session failed to unsubscribe on topic: {0}")]
    UnsubscriptionFailure(String),
}
