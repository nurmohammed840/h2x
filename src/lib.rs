#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

pub use bytes;
pub use h2;
pub use http;
pub use tokio_tls_listener;

mod graceful_shutdown;
mod request;
mod response;
mod server;

pub use graceful_shutdown::GracefulShutdown;
pub use request::*;
pub use response::*;
pub use server::*;

use bytes::Bytes;
use std::{
    future::{poll_fn, Future},
    task::{Context, Poll},
};

type BoxErr = Box<dyn std::error::Error + Send + Sync>;

/// Represents HTTP/2 result operation.
///
/// This type uses the [h2::Error] as the error type.
/// This allows functions returning this Result type to propagate errors specific to the [h2] library.
pub type Result<T, E = h2::Error> = std::result::Result<T, E>;

fn io_err(error: impl Into<BoxErr>) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, error)
}

/// Serving incoming connections and handling streams using the provided callbacks.
pub trait Incoming: Clone + Send + 'static {
    /// Called for each stream within a connection and is responsible for processing the stream
    fn stream(self, req: Request, res: Response) -> impl Future<Output = ()> + Send;

    /// Called when disconnected
    #[inline]
    fn close(self) -> impl Future<Output = ()> + Send {
        async {}
    }
}