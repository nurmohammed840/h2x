use h2_plus::{
    http::{Method, StatusCode},
    tokio_tls_listener::tokio_rustls::server::TlsStream,
    Conn, Server,
};
use http::{HeaderName, HeaderValue};
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
        println!("{:?}", req.uri.path());

        match (req.method.clone(), req.uri.path()) {
            (Method::GET, "/") => {}
            (Method::GET, "/test") => {
                let body = "Hello, World\n".repeat(1024 * 100);

                res.status = StatusCode::OK;
                res.headers.append(
                    HeaderName::from_static("content-type"),
                    HeaderValue::from_static("text/html"),
                );
                // res.headers.append(
                //     HeaderName::from_static("content-length"),
                //     HeaderValue::from(body.len()),
                // );
                println!("{:?}", res.write(body).await);
            }
            _ => {
                res.status = StatusCode::OK;
            }
        }
    }
}
