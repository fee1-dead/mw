mod ua;
use reqwest::header::InvalidHeaderValue;
pub use ua::UA;

mod client;
pub use client::{Client, ClientBuilder, Params};

mod cont;

/// The error type for this crate.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// An error that came from the `reqwest` library.
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    /// Should usually come from when setting a user agent.
    #[error(transparent)]
    InvalidHeaderValue(#[from] InvalidHeaderValue),
    /// The bot cannot log in.
    #[error("failed to log in")]
    Unauthorized,
}

/// Result type that uses the error type of this crate by default.
pub type Result<T, E = Error> = std::result::Result<T, E>;

#[cfg(test)]
mod tests {}
