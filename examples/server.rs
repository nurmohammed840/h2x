use h2_plus::{
    http::{Method, StatusCode},
    tokio_tls_listener::tokio_rustls::server::TlsStream,
    Conn, Server,
};
use http::{HeaderName, HeaderValue};
use tokio::net::TcpStream;
use tracing::info;

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .finish();

    // std::env::set_var("SSLKEYLOGFILE", "./SSLKEYLOGFILE.log");
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let mut server = Server::bind(
        "127.0.0.1:4433",
        "./examples/cert.pem",
        "./examples/key.pem",
    )
    .await
    .unwrap();

    info!("listening: {:?}", server.listener.local_addr());

    loop {
        let Ok((conn, _addr)) = server.accept().await else { continue };
        tokio::spawn(route(conn));
    }
}

async fn route(mut conn: Conn<TlsStream<TcpStream>>) {
    while let Some(Ok((req, mut res))) = conn.accept().await {
        info!("{:?}", req.uri.path());

        match (req.method.clone(), req.uri.path()) {
            (Method::GET, "/") => {}
            (Method::GET, "/test") => {
                let body = vec![0; 409601];

                res.status = StatusCode::OK;
                res.headers.append(
                    HeaderName::from_static("content-type"),
                    HeaderValue::from_static("text/html"),
                );
                // res.headers.append(
                //     HeaderName::from_static("content-length"),
                //     HeaderValue::from(body.len()),
                // );
                info!("{:?}", res.write(body).await);
            }
            _ => {
                res.status = StatusCode::OK;
            }
        }
    }
}
