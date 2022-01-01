use anyhow::{Error, Result};
use futures::stream::{self, StreamExt};
use hyper::service::{make_service_fn, service_fn};
use hyper::{body::Bytes, Body, Method, Request, Response, Server};

const CHUNK: [u8; 4096] = [42; 4096];

/// endless stream of bytes
async fn speedtest(_: Request<Body>) -> Result<Response<Body>> {
    // let body_stream = stream::iter(chunk).cycle().map(|b| Ok::<_, Error>(b)); // endless stream
    let chunks = (0..100).map(|_| &CHUNK);
    let body_stream = stream::iter(chunks)
        .cycle()
        .map(|b| Ok::<_, Error>(Bytes::from_static(b))); // endless stream
    Ok(Response::new(Body::wrap_stream(body_stream)))
}

async fn handle(req: Request<Body>) -> Result<Response<Body>> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/speedtest") => speedtest(req).await,
        _ => Ok(Response::new(Body::from("hi"))),
    }
}

async fn shutdown_signal() {
    // Wait for the CTRL+C signal
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    pretty_env_logger::init();

    // For every connection, we must make a `Service` to handle all
    // incoming HTTP requests on said connection.
    let make_svc = make_service_fn(|_conn| {
        // This is the `Service` that will handle the connection.
        // `service_fn` is a helper to convert a function that
        // returns a Response into a `Service`.
        async { Ok::<_, Error>(service_fn(handle)) }
    });

    let addr = ([127, 0, 0, 1], 3000).into();

    let server = Server::bind(&addr)
        .serve(make_svc)
        .with_graceful_shutdown(shutdown_signal());

    println!("Listening on http://{}", addr);

    server.await?;

    Ok(())
}
