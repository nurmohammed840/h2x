#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

pub use bytes;
pub use h2;
pub use http;
pub use tokio_tls_listener;

mod listener;
mod request;
mod response;
mod server;

#[doc(hidden)]
pub mod wait_group;

pub use listener::*;
pub use request::*;
pub use response::*;
pub use server::*;

use bytes::Bytes;
use std::{
    future::{poll_fn, Future},
    task::{Context, Poll},
};

type DynErr = Box<dyn std::error::Error + Send + Sync>;

/// Represents HTTP/2 result operation.
///
/// This type uses the [h2::Error] as the error type.
/// This allows functions returning this Result type to propagate errors specific to the [h2] library.
pub type Result<T, E = h2::Error> = std::result::Result<T, E>;

fn io_err(error: impl Into<DynErr>) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, error)
}
