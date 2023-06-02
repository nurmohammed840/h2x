use h2x::{
    http::{HeaderValue, Method, StatusCode},
    *,
};
use std::{
    fs,
    io::Result,
    net::SocketAddr,
    ops::ControlFlow,
    sync::atomic::{AtomicBool, Ordering},
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

    static IS_RUNNING: AtomicBool = AtomicBool::new(true);

    tokio::spawn(async {
        tokio::signal::ctrl_c().await.unwrap();
        IS_RUNNING.store(false, Ordering::Relaxed);
    });

    let c = server
        .serve_with_graceful_shutdown(
            |addr| {
                if !IS_RUNNING.load(Ordering::Acquire) {
                    return ControlFlow::Break(());
                }
                println!("[{addr}] New connection");
                ControlFlow::Continue(Some(addr))
            },
            |_conn, addr, req, res| handler(addr, req, res),
        )
        .await;

    println!("\nClosing...");
    Ok(c.await)
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
