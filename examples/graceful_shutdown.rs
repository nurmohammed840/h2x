use h2x::{
    http::{HeaderValue, Method, StatusCode},
    shutdown::ShutDownState,
    *,
};
use std::{fs, future::Future, io::Result, net::SocketAddr, pin::pin, task::Poll};

#[tokio::main]
async fn main() -> Result<()> {
    // std::env::set_var("SSLKEYLOGFILE", "./SSLKEYLOGFILE.log");
    let server = Server::bind(
        "127.0.0.1:4433",
        &mut fs::read("examples/cert.pem")?.as_slice(),
        &mut fs::read("examples/key.pem")?.as_slice(),
    )
    .await
    .unwrap();

    println!("Goto: https://{}/", server.listener.local_addr()?);

    let state = ShutDownState::new();

    let (server, wait_for_shutdown) = server.serve_with_graceful_shutdown(
        state,
        |addr| async move {
            println!("[{addr}] New connection");
            Some(addr)
        },
        |_conn, addr, req, res| handler(addr, req, res),
    );
    {
        let mut server = pin!(server);
        let mut signal = pin!(tokio::signal::ctrl_c());
        std::future::poll_fn(|cx| {
            if signal.as_mut().poll(cx).is_ready() {
                return Poll::Ready(());
            }
            server.as_mut().poll(cx).map(|_| ())
        })
        .await;
    }
    println!("\nClosing...");
    Ok(wait_for_shutdown.await)
}

async fn handler(addr: SocketAddr, req: Request, mut res: Response) -> h2x::Result<()> {
    println!("[{addr}] {req:#?}");

    res.headers
        .append("access-control-allow-origin", HeaderValue::from_static("*"));

    res.headers
        .append("content-type", HeaderValue::from_static("text/html"));

    match (req.method.clone(), req.uri.path()) {
        (Method::GET, "/") => {
            res.write(fs::read_to_string("examples/index.html").unwrap())
                .await
        }
        (Method::GET, "/test") => {
            let body = "Hello, World!\n".repeat(10);

            res.headers
                .append("content-length", HeaderValue::from(body.len()));

            res.write(body).await
        }
        (method, path) => {
            res.status = StatusCode::NOT_FOUND;
            res.write(format!("{method} {path}")).await
        }
    }
}
