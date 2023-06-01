use super::*;
use std::{io::BufRead, net::SocketAddr, ops};
use tokio::{
    io::{self, AsyncRead, AsyncWrite},
    net::{TcpStream, ToSocketAddrs},
};
use tokio_tls_listener::{tls_config, tokio_rustls::server::TlsStream, TlsListener};

/// It is used to accept incoming HTTP/2 connections.
pub struct Listener(
    /// The underlying [TlsListener] instance that provides secure transport layer functionality
    #[doc(hidden)]
    pub TlsListener,
);

impl Listener {
    /// Bind and listen for incoming connections on the specified address.
    pub async fn bind(
        addr: impl ToSocketAddrs,
        certs: &mut dyn BufRead,
        key: &mut dyn BufRead,
    ) -> Result<Self, DynErr> {
        let mut conf = tls_config(certs, key)?;
        conf.alpn_protocols = vec![b"h2".to_vec()];
        if cfg!(debug_assertions) && std::env::var("SSLKEYLOGFILE").is_ok() {
            conf.key_log = std::sync::Arc::new(tokio_tls_listener::rustls::KeyLogFile::new());
        }
        Ok(Self(TlsListener::bind(addr, conf).await?))
    }

    /// Accept incoming connections
    #[inline]
    pub async fn accept(&mut self) -> io::Result<(Conn<TlsStream<TcpStream>>, SocketAddr)> {
        let (stream, addr) = self.0.accept_tls().await?;
        let conn = Conn::handshake(stream).await.map_err(io_err)?;
        Ok((conn, addr))
    }
}

impl ops::Deref for Listener {
    type Target = TlsListener;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Represents an HTTP/2 connection.
#[derive(Debug)]
pub struct Conn<IO>(#[doc(hidden)] pub h2::server::Connection<IO, Bytes>);

impl<IO> Conn<IO>
where
    IO: Unpin + AsyncRead + AsyncWrite,
{
    /// Creates a new configured HTTP/2 server with default configuration.
    #[inline]
    pub async fn handshake(io: IO) -> Result<Conn<IO>> {
        h2::server::handshake(io).await.map(Self)
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
        self.0.poll_accept(cx).map(|event| {
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
}

impl<IO> ops::Deref for Conn<IO> {
    type Target = h2::server::Connection<IO, Bytes>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<IO> ops::DerefMut for Conn<IO> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
