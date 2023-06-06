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
h2x = "0.3"
```

### Example 

You can run this example with: `cargo run --example hello_world`

```rust no_run
use h2x::*;
use http::{Method, StatusCode};
use std::{fs, io::Result, ops::ControlFlow};

#[tokio::main]
async fn main() -> Result<()> {
    let addr = "127.0.0.1:4433";
    let cert = fs::read("examples/cert.pem")?;
    let key = fs::read("examples/key.pem")?;

    println!("Goto: https://{addr}");

    Server::bind(addr, &mut &*cert, &mut &*key).await.unwrap().serve(
        |addr| async move {
            println!("[{addr}] NEW CONNECTION");
            ControlFlow::Continue(Some(addr))
        },
        |_conn, _addr, mut req, mut res| async move {
            match (&req.method, req.uri.path()) {
                (&Method::GET, "/") => res.write("<H1>Hello, World</H1>").await,
                _ => {
                    // Echo
                    res.status = StatusCode::NOT_FOUND;
                    let mut stream = res.send_stream()?;
                    stream.write(format!("{req:#?}")).await?;
                    while let Some(bytes) = req.data().await {
                        stream.write(bytes?).await?;
                    }
                    stream.end()
                }
            }
        },
        |addr| async move { println!("[{addr}] CONNECTION CLOSE") },
    )
    .await;

    Ok(())
}
```

For more examples, see [./examples](https://github.com/nurmohammed840/h2x/tree/master/examples) directory.
