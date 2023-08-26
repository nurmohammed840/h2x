use h2x::*;
use http::{Method, StatusCode};
use std::{error::Error, fs};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let cert = fs::read("examples/cert.pem")?;
    let key = fs::read("examples/key.pem")?;

    let server = Server::bind("127.0.0.1:4433", &mut &*cert, &mut &*key).await?;
    println!("Goto: https://{}", server.local_addr()?);
    loop {
        if let Ok((conn, addr)) = server.accept().await {
            println!("[{}] NEW CONNECTION", addr);

            conn.incoming(
                addr,
                |_, addr, mut req, mut res| async move {
                    println!("From: {addr} at {}", req.uri.path());

                    match (&req.method, req.uri.path()) {
                        (&Method::GET, "/") => res.write("<H1>Hello, World</H1>").await,
                        _ => {
                            // Echo
                            res.status = StatusCode::NOT_FOUND;
                            let mut stream = res.send_stream()?;
                            stream.write(format!("{req:#?}\n")).await?;
                            while let Some(bytes) = req.data().await {
                                stream.write(bytes?).await?;
                            }
                            stream.end()
                        }
                    }
                },
                |addr| async move { println!("[{addr}] CONNECTION CLOSE") },
            );
        }
    }
}
