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

type DynErr = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T, E = h2::Error> = std::result::Result<T, E>;
