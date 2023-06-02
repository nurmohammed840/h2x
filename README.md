## h2x

`h2x` is a Rust library that provides a wrapper around the [h2](https://github.com/hyperium/h2) crate, offering additional functionality and utility functions for working with the HTTP/2 protocol.

It aims to simplify the usage of the `h2` crate and provide a more ergonomic API for building HTTP/2 servers.

## Getting Started

To use `h2x` in your Rust project, add it as a dependency in your `Cargo.toml` file:

```toml
[dependencies]
h2x = "0.1"
```

### Example 

You can run this example with: `cargo run --example hello_world`

```rust no_run
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
            |addr| {
                println!("[{addr}] NEW CONNECTION");
                ControlFlow::Continue(Some(addr))
            },
            |_conn, addr, req, mut res| async move {
                println!("[{addr}] {req:#?}");
                let _ = match (req.method.clone(), req.uri.path()) {
                    (Method::GET, "/") => res.write("<H1>Hello, World</H1>").await,
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
```