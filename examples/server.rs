use h2_plus::{
    http::{Method, StatusCode},
    tokio_tls_listener::tokio_rustls::server::TlsStream,
    Conn, Request, Response, Server,
};
use http::HeaderValue;
use tokio::net::TcpStream;

#[tokio::main]
async fn main() {
    // std::env::set_var("SSLKEYLOGFILE", "./SSLKEYLOGFILE.log");
    let mut server = Server::bind(
        "127.0.0.1:4433",
        "./examples/cert.pem",
        "./examples/key.pem",
    )
    .await
    .unwrap();

    println!("listening: {:?}", server.listener.local_addr());

    loop {
        let Ok((conn, _addr)) = server.accept().await else { continue };
        tokio::spawn(acceptor(conn));
    }
}

async fn acceptor(mut conn: Conn<TlsStream<TcpStream>>) {
    while let Some(Ok((req, res))) = conn.accept().await {
        tokio::spawn(handler(req, res));
    }
}

async fn handler(req: Request, mut res: Response) -> h2_plus::Result<()> {
    res.headers
        .append("access-control-allow-origin", HeaderValue::from_static("*"));
    res.headers
        .append("content-type", HeaderValue::from_static("text/html"));

    match (req.method.clone(), req.uri.path()) {
        (Method::GET, "/") => {
            let data = std::fs::read_to_string("./examples/index.html").unwrap();
            res.write(data).await
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
