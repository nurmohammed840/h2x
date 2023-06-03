use crate::shutdown::{SignalShutdown, WaitGroup};

use super::*;
use std::{io::BufRead, net::SocketAddr, ops::ControlFlow, sync::Arc};
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
    pub fn serve_with_graceful_shutdown<State, Accept, Stream>(
        mut self,
        state: impl SignalShutdown,
        on_accept: impl FnOnce(SocketAddr) -> Accept + Clone,
        on_stream: fn(&mut Conn<TlsStream<TcpStream>>, State, Request, Response) -> Stream,
    ) -> (impl Future<Output = ()>, impl Future<Output = ()>)
    where
        State: Clone + Send + 'static,
        Accept: Future<Output = Option<State>>,
        Stream: Future + Send + 'static,
    {
        struct ShutdownOnDrop<T: SignalShutdown>(Arc<T>);

        impl<T: SignalShutdown> Drop for ShutdownOnDrop<T> {
            fn drop(&mut self) {
                self.0.shutdown();
            }
        }
        let state = Arc::new(state);
        let wg = ShutdownOnDrop(Arc::clone(&state));
        let server = async move {
            loop {
                let Ok((mut conn, addr)) = self.listener.accept().await else { continue };
                if wg.0.is_shutdown() {
                    break;
                }
                let on_accept = on_accept.clone();
                let Some(state) = on_accept(addr).await else { continue };
                let wg = Arc::clone(&wg.0);
                tokio::spawn(async move {
                    while let Some(Ok((req, res))) = conn.accept().await {
                        if wg.is_shutdown() {
                            conn.0.graceful_shutdown();
                        } else {
                            let wg = Arc::clone(&wg);
                            let state = state.clone();
                            let future = on_stream(&mut conn, state, req, res);
                            tokio::spawn(async move {
                                let _ = future.await;
                                drop(wg);
                            });
                        }
                    }
                    drop(wg);
                });
            }
            drop(wg);
        };
        (server, WaitGroup(state))
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
    pub async fn serve<State, Accept, Stream>(
        mut self,
        on_accept: impl FnOnce(SocketAddr) -> Accept + Clone,
        on_stream: fn(&mut Conn<TlsStream<TcpStream>>, State, Request, Response) -> Stream,
    ) where
        State: Clone + Send + 'static,
        Accept: Future<Output = ControlFlow<(), Option<State>>>,
        Stream: Future + Send + 'static,
        Stream::Output: Send,
    {
        loop {
            let Ok((mut conn, addr)) = self.listener.accept().await else { continue };
            let on_accept = on_accept.clone();
            let state = match on_accept(addr).await {
                ControlFlow::Continue(ctx) => match ctx {
                    Some(state) => state,
                    None => continue,
                },
                ControlFlow::Break(_) => break,
            };
            tokio::spawn(async move {
                while let Some(Ok((req, res))) = conn.accept().await {
                    let state = state.clone();
                    tokio::spawn(on_stream(&mut conn, state, req, res));
                }
            });
        }
    }
}
