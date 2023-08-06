use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
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
    pub fn incoming<State, Stream, Close>(
        mut self,
        state: State,
        on_stream: fn(&mut Self, State, Request, Response) -> Stream,
        on_close: fn(State) -> Close,
    ) where
        State: Clone + Send + 'static,
        Stream: Future + Send + 'static,
        Stream::Output: Send,
        Close: Future + Send + 'static,
    {
        tokio::spawn(async move {
            while let Some(Ok((req, res))) = self.inner.accept().await {
                if self.is_closed.load(Ordering::Acquire) {
                    self.inner.graceful_shutdown();
                } else {
                    let is_closed = Arc::clone(&self.is_closed);
                    let state = state.clone();
                    let future = on_stream(&mut self, state, req, res);
                    tokio::spawn(async move {
                        let _ = future.await;
                        drop(is_closed);
                    });
                }
            }
            on_close(state).await;
            drop(self.is_closed);
        });
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
