use super::*;
use h2::server::Connection;
use std::{
    io::BufRead,
    net::SocketAddr,
    ops::ControlFlow,
    sync::{
        atomic::{self, AtomicBool},
        Arc,
    },
};
use tokio::net::{TcpStream, ToSocketAddrs};
use tokio_tls_listener::tokio_rustls::server::TlsStream;

/// The [Server] represents an HTTP server that listens for incoming connections.
pub struct Server {
    /// [Listener] is responsible for accepting incoming connections.
    pub listener: Listener,
}

impl From<Listener> for Server {
    fn from(listener: Listener) -> Self {
        Self { listener }
    }
}

impl Server {
    /// Binds the [Server] to the specified address.
    pub async fn bind(
        addr: impl ToSocketAddrs,
        certs: &mut dyn BufRead,
        key: &mut dyn BufRead,
    ) -> Result<Self, DynErr> {
        Listener::bind(addr, certs, key).await.map(Self::from)
    }

    /// Graceful shutdown allow existing connections to complete before shutting down the server.
    /// It returns a future that resolves when all existing connections have completed.
    pub async fn serve_with_graceful_shutdown<State, Stream>(
        mut self,
        on_accept: impl FnOnce(SocketAddr) -> ControlFlow<(), Option<State>> + Clone + 'static,
        on_stream: fn(
            &mut Connection<TlsStream<TcpStream>, Bytes>,
            State,
            Request,
            Response,
        ) -> Stream,
    ) -> impl Future<Output = ()>
    where
        State: Clone + Send + 'static,
        Stream: Future + Send + 'static,
    {
        let waitgroup = Arc::new(AtomicBool::new(false));
        loop {
            let Ok((mut conn, addr)) = self.listener.accept().await else { continue };
            let on_accept = on_accept.clone();
            let state = match on_accept(addr) {
                ControlFlow::Continue(ctx) => match ctx {
                    Some(state) => state,
                    None => continue,
                },
                ControlFlow::Break(_) => break,
            };
            let wg = Arc::clone(&waitgroup);
            tokio::spawn(async move {
                while let Some(Ok((req, res))) = conn.accept().await {
                    if wg.load(atomic::Ordering::Acquire) {
                        conn.0.graceful_shutdown();
                    } else {
                        let wg = Arc::clone(&wg);
                        let state = state.clone();
                        let future = on_stream(&mut conn.0, state, req, res);
                        tokio::spawn(async move {
                            let _ = future.await;
                            drop(wg);
                        });
                    }
                }
                drop(wg);
            });
        }
        waitgroup.store(true, atomic::Ordering::Relaxed);
        std::future::poll_fn(move |cx| {
            if Arc::strong_count(&waitgroup) == 1 {
                return Poll::Ready(());
            }
            std::thread::yield_now();
            cx.waker().wake_by_ref();
            Poll::Pending
        })
    }

    /// Starts serving incoming connections and handling streams using the provided callbacks.
    ///
    /// ### `on_accept`
    ///
    /// Called once for each accepted connection. This closure that takes a [SocketAddr]
    /// and returns a [ControlFlow] enum indicating how the server should handle the connection.
    ///
    /// The different variants of control-flow are:
    ///
    /// - `ControlFlow::Continue(Some(<State>))`: Accepts the HTTP connection with the provided state.
    /// - `ControlFlow::Continue(None)`: Rejects the HTTP connection.
    /// - `ControlFlow::Break(())`: Stopping the server from accepting further connections.
    ///
    /// ### `on_stream`
    ///
    /// Called for each stream within a connection and is responsible for processing the stream.
    pub async fn serve<State, Stream>(
        mut self,
        on_accept: impl FnOnce(SocketAddr) -> ControlFlow<(), Option<State>> + Clone + 'static,
        on_stream: fn(
            &mut Connection<TlsStream<TcpStream>, Bytes>,
            State,
            Request,
            Response,
        ) -> Stream,
    ) where
        State: Clone + Send + 'static,
        Stream: Future + Send + 'static,
        Stream::Output: Send,
    {
        loop {
            let Ok((mut conn, addr)) = self.listener.accept().await else { continue };
            let on_accept = on_accept.clone();
            let state = match on_accept(addr) {
                ControlFlow::Continue(ctx) => match ctx {
                    Some(state) => state,
                    None => continue,
                },
                ControlFlow::Break(_) => break,
            };
            tokio::spawn(async move {
                while let Some(Ok((req, res))) = conn.accept().await {
                    let state = state.clone();
                    tokio::spawn(on_stream(&mut conn.0, state, req, res));
                }
            });
        }
    }
}
