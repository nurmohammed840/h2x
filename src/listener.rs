use super::*;
use std::{io::BufRead, net::SocketAddr, ops};
use tokio::{
    io::{self, AsyncRead, AsyncWrite},
    net::{TcpStream, ToSocketAddrs},
};
use tokio_tls_listener::{tls_config, tokio_rustls::server::TlsStream, TlsListener};

pub struct Listener(#[doc(hidden)] pub TlsListener);

impl Listener {
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

#[derive(Debug)]
pub struct Conn<IO>(#[doc(hidden)] pub h2::server::Connection<IO, Bytes>);

impl<IO> Conn<IO>
where
    IO: Unpin + AsyncRead + AsyncWrite,
{
    #[inline]
    pub async fn handshake(io: IO) -> Result<Conn<IO>> {
        h2::server::handshake(io).await.map(Self)
    }

    #[inline]
    pub fn accept(&mut self) -> Accept<IO> {
        Accept { this: self }
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

pub struct Accept<'a, IO> {
    this: &'a mut Conn<IO>,
}

impl<IO> Future for Accept<'_, IO>
where
    IO: Unpin + AsyncRead + AsyncWrite,
{
    type Output = Option<Result<(Request, Response)>>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.this.poll_accept(cx)
    }
}
