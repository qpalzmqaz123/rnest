pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Mutex lock error: `{0}`")]
    Lock(String),
    #[error("Type mismatch: `{0}`")]
    TypeMismatch(String),
    #[error("Key not found: `{0}`")]
    KeyNotFound(String),
    #[error("Circular dependency: `{0}`")]
    CircularDependency(String),
    #[error("Try to inject private provider: `{0}`")]
    InjectPrivateProvider(String),
    #[error("User error: `{0}`")]
    User(String),
    #[error("Unknown error: `{0}`")]
    Unknown(String),
}

impl<T> From<std::sync::PoisonError<T>> for Error {
    fn from(e: std::sync::PoisonError<T>) -> Self {
        Error::Lock(format!("Mutex lock error: {}", e))
    }
}

impl From<Box<dyn std::error::Error>> for Error {
    fn from(e: Box<dyn std::error::Error>) -> Self {
        Error::Unknown(format!("{}", e))
    }
}
