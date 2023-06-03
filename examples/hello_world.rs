use h2x::*;
use http::{Method, StatusCode};
use std::{fs, io::Result, ops::ControlFlow};

#[tokio::main]
async fn main() -> Result<()> {
    let addr = "127.0.0.1:4433";
    let cert = fs::read("examples/cert.pem")?;
    let key = fs::read("examples/key.pem")?;

    println!("Goto: https://{addr}");

    Server::bind(addr, &mut &*cert, &mut &*key)
        .await
        .unwrap()
        .serve(
            |addr| async move {
                println!("[{addr}] NEW CONNECTION");
                ControlFlow::Continue(Some(addr))
            },
            |_conn, addr, req, mut res| async move {
                println!("From: {addr} at {}", req.uri.path());
                let _ = match (&req.method, req.uri.path()) {
                    (&Method::GET, "/") => res.write("<H1>Hello, World</H1>").await,
                    _ => {
                        res.status = StatusCode::NOT_FOUND;
                        res.write(format!("{req:#?}")).await
                    }
                };
            },
            |addr| async move { println!("[{addr}] CONNECTION CLOSE") },
        )
        .await;

    Ok(())
}
