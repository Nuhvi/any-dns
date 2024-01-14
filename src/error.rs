//! Main Crate Error

#[derive(thiserror::Error, Debug)]
/// crate error enum.
pub enum Error {
    /// For starter, to remove as code matures.
    #[error("Generic error: {0}")]
    Generic(String),
    /// For starter, to remove as code matures.
    #[error("Static error: {0}")]
    Static(&'static str),

    #[error(transparent)]
    /// Transparent [std::io::Error]
    IO(#[from] std::io::Error),
}

// Alias Result to be the crate Result.
pub type Result<T, E = Error> = core::result::Result<T, E>;
