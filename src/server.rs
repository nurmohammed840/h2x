use super::*;
use std::{net::SocketAddr, ops, path::Path, sync::Arc};
use tokio::{
    io::{self, AsyncRead, AsyncWrite},
    net::{TcpStream, ToSocketAddrs}, task,
};
use tokio_tls_listener::{rustls, tokio_rustls::server::TlsStream, TlsListener};

/// An HTTP/2 server that listens for incoming connections.
pub struct Server {
    #[doc(hidden)]
    /// The underlying [TlsListener] instance that provides secure transport layer functionality
    pub listener: TlsListener,
}

impl Server {
    /// Default TLS server configuration.  
    pub fn config(
        key: impl AsRef<Path>,
        cert: impl AsRef<Path>,
    ) -> io::Result<rustls::ServerConfig> {
        let mut conf = tokio_tls_listener::load_tls_config(key, cert)?;
        conf.alpn_protocols = vec![b"h2".to_vec()];
        #[cfg(debug_assertions)]
        if std::env::var("SSLKEYLOGFILE").is_ok() {
            conf.key_log = std::sync::Arc::new(rustls::KeyLogFile::new());
        }
        Ok(conf)
    }

    /// Bind and listen for incoming connections on the specified address.
    pub async fn bind(
        addr: impl ToSocketAddrs,
        conf: impl Into<Arc<rustls::ServerConfig>>,
    ) -> io::Result<Self> {
        Ok(Self {
            listener: TlsListener::bind(addr, conf).await?,
        })
    }

    /// This method wraps the current server instance and returns a [GracefulShutdown]
    /// instance that allows for a controlled shutdown of the server.
    pub fn with_graceful_shutdown(self) -> GracefulShutdown<Self> {
        GracefulShutdown::new(self)
    }

    /// Accept incoming connections
    #[inline]
    pub async fn accept(&self) -> io::Result<(Conn<TlsStream<TcpStream>>, SocketAddr)> {
        let (stream, addr) = self.listener.accept_tls().await?;
        let conn = Conn::handshake(stream).await.map_err(io_err)?;
        Ok((conn, addr))
    }
}

/// Represents an HTTP/2 connection.
#[derive(Debug)]
pub struct Conn<IO> {
    inner: h2::server::Connection<IO, Bytes>,
}

impl<IO> Conn<IO>
where
    IO: Unpin + AsyncRead + AsyncWrite,
{
    /// Creates a new configured HTTP/2 server with default configuration.
    #[inline]
    pub async fn handshake(io: IO) -> Result<Conn<IO>> {
        h2::server::handshake(io).await.map(|inner| Self { inner })
    }

    /// Accept a new incoming stream on the HTTP/2 connection
    pub async fn accept(&mut self) -> Option<Result<(Request, Response)>> {
        poll_fn(|cx| self.poll_accept(cx)).await
    }

    #[doc(hidden)]
    pub fn poll_accept(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<(Request, Response)>>> {
        self.inner.poll_accept(cx).map(|event| {
            event.map(|accept| {
                accept.map(|(req, sender)| {
                    let (head, body) = req.into_parts();
                    let request = Request { head, body };
                    let response = Response {
                        status: http::StatusCode::default(),
                        headers: http::HeaderMap::default(),
                        sender,
                    };
                    (request, response)
                })
            })
        })
    }

    /// Starts serving incoming connections and handling streams using the provided callbacks.
    ///
    /// ### `on_stream`
    ///
    /// Called for each stream within a connection and is responsible for processing the stream.
    ///
    /// ### `on_close`
    ///
    /// Called when disconnected.
    ///
    /// ## Example
    ///
    /// ```no_run
    /// use h2x::*;
    /// use http::{Method, StatusCode};
    /// use std::{io, net::SocketAddr};
    ///
    /// #[derive(Clone)]
    /// struct Service {
    ///     addr: SocketAddr,
    /// }
    ///
    /// impl Incoming for Service {
    ///     async fn stream(self, req: Request, mut res: Response) {
    ///         println!("From: {} at {}", self.addr, req.uri.path());
    ///         let _ = match (&req.method, req.uri.path()) {
    ///             (&Method::GET, "/") => res.write("<H1>Hello, World</H1>").await,
    ///             _ => {
    ///                 res.status = StatusCode::NOT_FOUND;
    ///                 res.write(format!("{req:#?}\n")).await
    ///             }
    ///         };
    ///     }
    ///
    ///     async fn close(self) {
    ///         println!("[{}] CONNECTION CLOSE", self.addr)
    ///     }
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() -> io::Result<()> {
    ///     let conf = Server::config("examples/key.pem", "examples/cert.pem")?;
    ///     let server = Server::bind("127.0.0.1:4433", conf).await?;
    ///     println!("Goto: https://{}", server.local_addr()?);
    ///
    ///     loop {
    ///         if let Ok((conn, addr)) = server.accept().await {
    ///             println!("[{}] NEW CONNECTION", addr);
    ///             conn.incoming(Service { addr });
    ///         }
    ///     }
    /// }
    /// ```
    pub fn incoming(mut self, _s: impl Incoming) -> task::JoinHandle<()>
    where
        IO: Send + 'static,
    {
        tokio::spawn(async move {
            while let Some(Ok((req, res))) = self.accept().await {
                let state = _s.clone();
                tokio::spawn(state.stream(req, res));
            }
            _s.close().await;
        })
    }
}

impl ops::Deref for Server {
    type Target = TlsListener;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.listener
    }
}

impl<IO> ops::Deref for Conn<IO> {
    type Target = h2::server::Connection<IO, Bytes>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<IO> ops::DerefMut for Conn<IO> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
