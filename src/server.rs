use crate::wait_group::WaitGroup;

use super::*;
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
    ///
    /// ### Returns Two Futures:
    ///
    /// - First one is for server.
    /// - Second future will resolves when all existing connections have finished.
    ///
    /// ## Example
    ///
    /// ```no_run
    /// use h2x::Server;
    /// use std::{fs, ops::ControlFlow};
    ///
    /// # async fn run() -> std::io::Result<()> {
    /// let addr = "127.0.0.1:4433";
    /// let cert = fs::read("cert.pem")?;
    /// let key = fs::read("key.pem")?;
    ///
    /// println!("Goto: https://{addr}");
    ///
    /// let (server, wait_for_shutdown) = Server::bind(addr, &mut &*cert, &mut &*key).await.unwrap()
    ///     .serve_with_graceful_shutdown(
    ///         |addr| async move {
    ///             println!("[{addr}] NEW CONNECTION");
    ///             ControlFlow::Continue(Some(addr))
    ///         },
    ///         |_conn, _addr, req, res| async move {
    ///             let _ = res.write(format!("{req:#?}")).await;
    ///         },
    ///         |addr| async move { println!("[{addr}] CONNECTION CLOSE") },
    ///     );
    ///
    /// // Close the running server on `CTRL + C`
    /// tokio::select! {
    ///     _ = tokio::signal::ctrl_c() => {}
    ///     _ = server => {}
    /// }
    /// println!("\nClosing...");
    /// wait_for_shutdown.await;
    /// # Ok(())
    /// # }
    /// ```
    pub fn serve_with_graceful_shutdown<State, Accept, Stream, Close>(
        mut self,
        on_accept: impl FnOnce(SocketAddr) -> Accept + Clone,
        on_stream: fn(&mut Conn<TlsStream<TcpStream>>, State, Request, Response) -> Stream,
        on_close: fn(State) -> Close,
    ) -> (impl Future<Output = ()>, impl Future<Output = ()>)
    where
        State: Clone + Send + 'static,
        Accept: Future<Output = ControlFlow<(), Option<State>>>,
        Stream: Future + Send + 'static,
        Close: Future + Send + 'static,
    {
        struct ShutdownOnDrop(Arc<AtomicBool>);
        impl Drop for ShutdownOnDrop {
            fn drop(&mut self) {
                self.0.store(true, atomic::Ordering::Relaxed);
            }
        }
        let state = Arc::new(AtomicBool::new(false));
        let wg = ShutdownOnDrop(Arc::clone(&state));
        let server = async move {
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
                let wg = Arc::clone(&wg.0);
                tokio::spawn(async move {
                    while let Some(Ok((req, res))) = conn.accept().await {
                        if wg.load(atomic::Ordering::Acquire) {
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
                    on_close(state).await;
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
    ///
    /// ### `on_close`
    ///
    /// Called when disconnected.
    ///
    /// ## Example
    ///
    /// ```no_run
    /// use h2x::Server;
    /// use std::{fs, ops::ControlFlow};
    ///
    /// # async fn run() -> std::io::Result<()> {
    /// let addr = "127.0.0.1:4433";
    /// let cert = fs::read("cert.pem")?;
    /// let key = fs::read("key.pem")?;
    ///
    /// println!("Goto: https://{addr}");
    ///
    /// Server::bind(addr, &mut &*cert, &mut &*key).await.unwrap().serve(
    ///     |addr| async move {
    ///         println!("[{addr}] NEW CONNECTION");
    ///         ControlFlow::Continue(Some(addr))
    ///     },
    ///     |_conn, _addr, req, res| async move {
    ///         let _ = res.write(format!("{req:#?}")).await;
    ///     },
    ///     |addr| async move { println!("[{addr}] CONNECTION CLOSE") },
    /// )
    /// .await;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn serve<State, Accept, Stream, Close>(
        mut self,
        on_accept: impl FnOnce(SocketAddr) -> Accept + Clone,
        on_stream: fn(&mut Conn<TlsStream<TcpStream>>, State, Request, Response) -> Stream,
        on_close: fn(State) -> Close,
    ) where
        State: Clone + Send + 'static,
        Accept: Future<Output = ControlFlow<(), Option<State>>>,
        Stream: Future + Send + 'static,
        Stream::Output: Send,
        Close: Future + Send + 'static,
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
                on_close(state).await;
            });
        }
    }
}
