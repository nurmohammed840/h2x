use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream, task,
};
use tokio_tls_listener::tokio_rustls::server::TlsStream;

use super::*;
use std::{
    future, io,
    net::SocketAddr,
    ops::{Deref, DerefMut},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

/// It allows gracefully shutdown capabilities for a server.
///
/// created from [`Server::with_graceful_shutdown()`] method.
pub struct GracefulShutdown<T> {
    is_closed: Arc<AtomicBool>,
    inner: T,
}

impl<T> GracefulShutdown<T> {
    pub(crate) fn new(inner: T) -> Self {
        Self {
            inner,
            is_closed: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Returns the current number of active connections being served.
    ///
    /// This method retrieves the count of active connections that the server is currently processing.
    pub fn num_of_conn(&self) -> usize {
        Arc::strong_count(&self.is_closed)
    }
}

impl GracefulShutdown<Server> {
    /// Accept incoming connections
    #[inline]
    pub async fn accept(
        &self,
    ) -> io::Result<(GracefulShutdown<Conn<TlsStream<TcpStream>>>, SocketAddr)> {
        self.inner.accept().await.map(|(inner, addr)| {
            let is_closed = Arc::clone(&self.is_closed);
            (GracefulShutdown { inner, is_closed }, addr)
        })
    }

    /// After calling this method, the server will stop accepting
    /// new connections and will eventually complete ongoing requests before
    /// shutting down.
    ///
    /// During the graceful shutdown process, the server will complete ongoing
    /// requests before closing the active connections. Once all active connections
    /// are served and no new connections are accepted, the server will completely
    /// shut down.
    pub fn shutdown(self) -> impl Future {
        self.is_closed.store(true, Ordering::Relaxed);
        future::poll_fn(move |cx| {
            // spin loop
            if Arc::strong_count(&self.is_closed) == 1 {
                return Poll::Ready(());
            }
            std::thread::yield_now();
            cx.waker().wake_by_ref();
            Poll::Pending
        })
    }
}

impl<IO> GracefulShutdown<Conn<IO>>
where
    IO: Unpin + AsyncRead + AsyncWrite + Send + 'static,
{
    /// See [`Conn::incoming`]
    pub fn incoming(mut self, _s: impl Incoming) -> task::JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(Ok((req, res))) = self.inner.accept().await {
                if self.is_closed.load(Ordering::Acquire) {
                    self.inner.graceful_shutdown();
                } else {
                    let is_closed = Arc::clone(&self.is_closed);
                    let state = _s.clone();
                    tokio::spawn(async move {
                        state.stream(req, res).await;
                        drop(is_closed);
                    });
                }
            }
            _s.close().await;
            drop(self.is_closed);
        })
    }
}

impl<T> Deref for GracefulShutdown<T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for GracefulShutdown<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
