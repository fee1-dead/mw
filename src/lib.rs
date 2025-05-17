mod ua;
use reqwest::header::InvalidHeaderValue;
pub use ua::UA;

mod client;
pub use client::{Client, ClientBuilder, Params};

mod cont;

/// The error type for this crate
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    InvalidHeaderValue(#[from] InvalidHeaderValue),
    #[error("failed to log in")]
    Unauthorized,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[cfg(test)]
mod tests {}
