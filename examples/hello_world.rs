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
