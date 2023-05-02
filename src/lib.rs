// #![warn(missing_docs)]
#![doc = include_str!("../README.md")]

pub use bytes;
pub use h2;
pub use http;
pub use tokio_tls_listener;

mod request;
mod response;
mod server;

pub use request::*;
pub use response::*;
pub use server::*;

use bytes::Bytes;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

type DynErr = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T, E = h2::Error> = std::result::Result<T, E>;

fn io_err(error: impl Into<DynErr>) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, error)
}
