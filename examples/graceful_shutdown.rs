use h2x::{
    http::{HeaderValue, Method, StatusCode},
    *,
};
use std::{fs, future::Future, io::Result, net::SocketAddr, pin::pin, task::Poll};

#[tokio::main]
async fn main() -> Result<()> {
    // std::env::set_var("SSLKEYLOGFILE", "./SSLKEYLOGFILE.log");
    let conf = Server::config("examples/key.pem", "examples/cert.pem")?;
    let server = Server::bind("127.0.0.1:4433", conf)
        .await?
        .with_graceful_shutdown();

    println!("Goto: https://{}/", server.local_addr()?);

    let serve = async {
        loop {
            if let Ok((conn, addr)) = server.accept().await {
                conn.incoming(Service { addr });
            }
        }
    };
    // Close the running server on `CTRL + C`
    {
        let mut serve = pin!(serve);
        let mut signal = pin!(tokio::signal::ctrl_c());
        std::future::poll_fn(|cx| {
            if signal.as_mut().poll(cx).is_ready() {
                return Poll::Ready(());
            }
            serve.as_mut().poll(cx)
        })
        .await;
    }
    println!("\nClosing...");
    server.shutdown().await;
    println!("Server closed!");
    Ok(())
}

#[derive(Clone)]
struct Service {
    addr: SocketAddr,
}

impl Incoming for Service {
    async fn stream(self, req: Request, res: Response) {
        let _ = handler(self.addr, req, res).await;
    }
    async fn close(self) {
        println!("[{}] CONNECTION CLOSE", self.addr);
    }
}

async fn handler(addr: SocketAddr, req: Request, mut res: Response) -> h2x::Result<()> {
    println!("From: {addr} at {}", req.uri.path());
    res.headers
        .append("access-control-allow-origin", HeaderValue::from_static("*"));

    res.headers
        .append("content-type", HeaderValue::from_static("text/html"));

    match (req.method.clone(), req.uri.path()) {
        (Method::GET, "/") => res.write(fs::read("examples/index.html").unwrap()).await,
        (Method::GET, "/test") => {
            let body = format!("{req:#?}");
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
