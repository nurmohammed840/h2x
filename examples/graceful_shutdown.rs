use h2x::{
    http::{HeaderValue, Method, StatusCode},
    *,
};
use std::{
    fs, future::Future, io::Result, net::SocketAddr, ops::ControlFlow, pin::pin, task::Poll,
};

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

    let (server, wait_for_shutdown) = server.serve_with_graceful_shutdown(
        |addr| async move {
            println!("[{addr}] NEW CONNECTION");
            ControlFlow::Continue(Some(addr))
        },
        |_conn, addr, req, res| handler(addr, req, res),
        |addr| async move { println!("[{addr}] CONNECTION CLOSE") },
    );
    {
        // Close the running server on `CTRL + C`
        let mut server = pin!(server);
        let mut signal = pin!(tokio::signal::ctrl_c());
        std::future::poll_fn(|cx| {
            if signal.as_mut().poll(cx).is_ready() {
                return Poll::Ready(());
            }
            server.as_mut().poll(cx)
        })
        .await;
    }
    println!("\nClosing...");
    wait_for_shutdown.await;
    Ok(println!("Server closed!"))
}

async fn handler(addr: SocketAddr, req: Request, mut res: Response) -> h2x::Result<()> {
    println!("From: {addr} at {}", req.uri.path());
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
        _ => {
            res.status = StatusCode::NOT_FOUND;
            res.write(format!("{req:#?}")).await
        }
    }
}
