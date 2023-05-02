use super::*;
use bytes::Bytes;
use std::{net::SocketAddr, path::Path, sync::Arc};
use tokio::{
    io::{self, AsyncRead, AsyncWrite},
    net::{TcpStream, ToSocketAddrs},
};
use tokio_tls_listener::{
    rustls::KeyLogFile, tls_config, tokio_rustls::server::TlsStream, TlsListener,
};

pub struct Server {
    pub listener: TlsListener,
}

impl Server {
    pub async fn bind(
        addr: impl ToSocketAddrs,
        cert: impl AsRef<Path>,
        key: impl AsRef<Path>,
    ) -> Result<Self, DynErr> {
        let mut conf = tls_config(cert, key)?;
        conf.alpn_protocols = vec![b"h2".to_vec()];
        if cfg!(debug_assertions) {
            conf.key_log = Arc::new(KeyLogFile::new());
        }
        Ok(Self {
            listener: TlsListener::bind(addr, conf).await?,
        })
    }

    #[inline]
    pub async fn accept(&mut self) -> io::Result<(Conn<TlsStream<TcpStream>>, SocketAddr)> {
        let (stream, addr): (TlsStream<TcpStream>, SocketAddr) = self.listener.accept_tls().await?;
        let conn = Conn::handshake(stream).await.map_err(io_err)?;
        Ok((conn, addr))
    }
}

#[derive(Debug)]
pub struct Conn<IO>(pub h2::server::Connection<IO, Bytes>);

impl<IO> Conn<IO>
where
    IO: Unpin + AsyncRead + AsyncWrite,
{
    #[inline]
    pub async fn handshake(io: IO) -> Result<Conn<IO>> {
        h2::server::handshake(io).await.map(Self)
    }

    #[inline]
    pub async fn accept(&mut self) -> Option<Result<(Request, Response)>> {
        Some(self.0.accept().await?.map(|(req, sender)| {
            let (head, body) = req.into_parts();
            let request = Request { head, body };
            let response = Response {
                status: http::StatusCode::default(),
                headers: http::HeaderMap::default(),
                sender,
            };
            (request, response)
        }))
    }
}

fn io_err(error: impl Into<DynErr>) -> io::Error {
    io::Error::new(io::ErrorKind::Other, error)
}
