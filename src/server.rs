use super::*;
use std::{io::BufRead, net::SocketAddr, ops};
use tokio::{
    io::{self, AsyncRead, AsyncWrite},
    net::{TcpStream, ToSocketAddrs},
};
use tokio_tls_listener::{tls_config, tokio_rustls::server::TlsStream, TlsListener};

/// An HTTP/2 server that listens for incoming connections.
pub struct Server {
    #[doc(hidden)]
    /// The underlying [TlsListener] instance that provides secure transport layer functionality
    pub listener: TlsListener,
}

impl Server {
    /// Bind and listen for incoming connections on the specified address.
    pub async fn bind(
        addr: impl ToSocketAddrs,
        certs: &mut dyn BufRead,
        key: &mut dyn BufRead,
    ) -> Result<Self, BoxErr> {
        let mut conf = tls_config(certs, key)?;
        conf.alpn_protocols = vec![b"h2".to_vec()];
        #[cfg(debug_assertions)]
        if std::env::var("SSLKEYLOGFILE").is_ok() {
            conf.key_log = std::sync::Arc::new(tokio_tls_listener::rustls::KeyLogFile::new());
        }
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
    /// use h2x::Server;
    /// use std::fs;
    ///
    /// # async fn _run() -> std::io::Result<()> {
    /// let cert = fs::read("cert.pem")?;
    /// let key = fs::read("key.pem")?;
    ///
    /// let server = Server::bind("127.0.0.1:4433", &mut &*cert, &mut &*key).await.unwrap();
    /// println!("Goto: https://{}", server.local_addr()?);
    ///
    /// loop {
    ///  if let Ok((conn, addr)) = server.accept().await {
    ///    conn.incoming(
    ///      addr,
    ///      |_, _, req, res| async move {
    ///        let _ = res.write(format!("{req:#?}")).await;
    ///      },
    ///      |addr| async move { println!("[{addr}] CONNECTION CLOSE") }
    ///    )
    ///  }
    /// }
    /// # }
    /// ```
    pub fn incoming<State, Stream, Close>(
        mut self,
        state: State,
        on_stream: fn(&mut Self, State, Request, Response) -> Stream,
        on_close: fn(State) -> Close,
    ) where
        IO: Send + 'static,
        State: Clone + Send + 'static,
        Stream: Future + Send + 'static,
        Stream::Output: Send,
        Close: Future + Send + 'static,
    {
        tokio::spawn(async move {
            while let Some(Ok((req, res))) = self.accept().await {
                let state = state.clone();
                tokio::spawn(on_stream(&mut self, state, req, res));
            }
            on_close(state).await;
        });
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
