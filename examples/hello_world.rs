use h2x::*;
use http::{Method, StatusCode};
use std::{fs, io::Result, ops::ControlFlow};

#[tokio::main]
async fn main() -> Result<()> {
    let server = Server::bind(
        "127.0.0.1:4433",
        &mut &*fs::read("examples/cert.pem")?,
        &mut &*fs::read("examples/key.pem")?,
    )
    .await
    .unwrap();

    println!("Goto: https://{}/", server.listener.local_addr()?);

    server
        .serve(
            |addr| async move {
                println!("[{addr}] NEW CONNECTION");
                ControlFlow::Continue(Some(addr))
            },
            |_conn, addr, req, mut res| async move {
                println!("[{addr}] {req:#?}");
                let _ = match (&req.method, req.uri.path()) {
                    (&Method::GET, "/") => res.write("<H1>Hello, World</H1>").await,
                    (method, path) => {
                        res.status = StatusCode::NOT_FOUND;
                        res.write(format!("{method} {path}")).await
                    }
                };
            },
        )
        .await;

    Ok(())
}
