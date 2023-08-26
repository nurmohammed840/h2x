## h2x

`h2x` provides a wrapper around the [h2](https://github.com/hyperium/h2) crate, offering additional functionality and utility functions for working with the HTTP/2 protocol.

It aims to simplify the usage of the `h2` crate and provide a more ergonomic API for building HTTP/2 servers.

## Goals

- Managing TCP connections
- TLS

If you only need HTTP/2 server and can't sacrifice any overhead this library is for you.

## Getting Started

To use `h2x` in your Rust project, add it as a dependency in your `Cargo.toml` file:

```toml
[dependencies]
h2x = "0.4"
```

### Example 

You can run this example with: `cargo run --example hello_world`

```rust no_run
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
```

For more examples, see [./examples](https://github.com/nurmohammed840/h2x/tree/master/examples) directory.
