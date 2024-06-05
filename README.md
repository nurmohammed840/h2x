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
h2x = "0.5"
```

### Example 

You can run this example with: `cargo run --example hello_world`

```rust no_run
use h2x::*;
use http::{Method, StatusCode};
use std::{io, net::SocketAddr};


#[derive(Clone)]
struct Service {
    addr: SocketAddr,
}

impl Incoming for Service {
    async fn stream(self, req: Request, mut res: Response) {
        println!("From: {} at {}", self.addr, req.uri.path());
        let _ = match (&req.method, req.uri.path()) {
            (&Method::GET, "/") => res.write("<H1>Hello, World</H1>").await,
            _ => {
                res.status = StatusCode::NOT_FOUND;
                res.write(format!("{req:#?}\n")).await
            }
        };
    }

    async fn close(self) {
        println!("[{}] CONNECTION CLOSE", self.addr)
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let conf = Server::config("examples/key.pem", "examples/cert.pem")?;
    let server = Server::bind("127.0.0.1:4433", conf).await?;

    println!("Goto: https://{}", server.local_addr()?);

    loop {
        if let Ok((conn, addr)) = server.accept().await {
            println!("[{}] NEW CONNECTION", addr);
            conn.incoming(Service { addr });
        }
    }
}

```

For more examples, see [./examples](https://github.com/nurmohammed840/h2x/tree/master/examples) directory.
