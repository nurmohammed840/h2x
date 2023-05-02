use h2_plus::{
    http::{Method, StatusCode},
    tokio_tls_listener::tokio_rustls::server::TlsStream,
    Conn, Server,
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
        tokio::spawn(route(conn));
    }
}

async fn route(mut conn: Conn<TlsStream<TcpStream>>) {
    while let Some(Ok((req, mut res))) = conn.accept().await {
        tokio::spawn(async move {
            let method = req.method.clone();
            let path = req.uri.path();
            println!("{method} {path}");

            match (method, path) {
                (Method::GET, "/") => {}
                (Method::GET, "/test") => {
                    let body = "Hello, World!\n".repeat(10);
                    res.status = StatusCode::OK;
                    res.headers
                        .append("access-control-allow-origin", HeaderValue::from_static("*"));
                    res.headers
                        .append("content-type", HeaderValue::from_static("text/plain"));
                    res.headers
                        .append("content-length", HeaderValue::from(body.len()));

                    let _ = res.write(body).await;
                }
                _ => {
                    res.status = StatusCode::NOT_FOUND;
                    let _ = res.send_headers();
                }
            }
        });
    }
}
