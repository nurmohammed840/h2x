use h2x::{
    http::{HeaderValue, Method, StatusCode},
    *,
};

use std::{
    fs,
    io::Result,
    ops::ControlFlow,
    sync::atomic::{AtomicUsize, Ordering},
};

#[tokio::main]
async fn main() -> Result<()> {
    // std::env::set_var("SSLKEYLOGFILE", "./SSLKEYLOGFILE.log");
    let server = Server::bind(
        "127.0.0.1:4433",
        &mut fs::read("examples/cert.pem")?.as_slice(),
        &mut fs::read("examples/key.pem")?.as_slice(),
    )
    .await
    .unwrap();

    println!("Goto: https://{}/", server.listener.local_addr()?);

    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let c = server
        .serve_with_graceful_shutdown(
            |addr| {
                let id = COUNTER.fetch_add(1, Ordering::Relaxed);
                if id == 1 {
                    return ControlFlow::Break(());
                }
                println!("Connection ID {id}: {:?}", addr);
                ControlFlow::Continue(Some(id))
            },
            |_conn, id, req, res| handler(id, req, res),
        )
        .await;

    println!("Closing...");
    Ok(c.await)
}

async fn handler(id: usize, req: Request, mut res: Response) -> h2x::Result<()> {
    println!("New request from {id}: {}", req.uri.path());

    res.headers
        .append("access-control-allow-origin", HeaderValue::from_static("*"));

    res.headers
        .append("content-type", HeaderValue::from_static("text/html"));

    match (req.method.clone(), req.uri.path()) {
        (Method::GET, "/") => {
            res.write(fs::read_to_string("examples/index.html").unwrap())
                .await
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

// mod tests {
//     #![allow(warnings)]
//     use super::*;
//     use std::{future::Future, pin::pin, time::Duration};

//     async fn test_name() {
//         let mut f = pin!(async {
//             // std::future::pending::<()>().await;
//             println!("Ok");
//             tokio::task::spawn_blocking(|| {
//                 std::thread::sleep(Duration::from_secs(2));
//             })
//             .await;
//             println!("Done!");
//         });
//         let mut fa = 0;
//         std::future::poll_fn(|cx| {
//             fa += 1;
//             println!("Polled!");
//             f.as_mut().poll(cx)
//         })
//         .await;
//         println!("{:?}", fa);
//     }

//     #[tokio::main]
//     async fn s() {
//         test_name().await;
//     }
//     #[test]
//     fn wda() {
//         s();
//     }
// }
