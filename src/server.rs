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

pub struct Server {
    pub listener: Listener,
}

impl Server {
    pub fn new(listener: Listener) -> Self {
        Self { listener }
    }

    pub async fn bind(
        addr: impl ToSocketAddrs,
        certs: &mut dyn BufRead,
        key: &mut dyn BufRead,
    ) -> Result<Self, DynErr> {
        Listener::bind(addr, certs, key).await.map(Self::new)
    }

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
        Stream::Output: Send,
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
                            future.await;
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
