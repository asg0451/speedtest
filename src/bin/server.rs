use anyhow::{Error, Result};
use clap::Parser;
use futures::stream::{self, StreamExt};
use hyper::service::{make_service_fn, service_fn};
use hyper::{body::Bytes, Body, Method, Request, Response, Server};

const KIB: usize = 1024;
const MIB: usize = 1024 * KIB;
const CHUNK: [u8; 1 * MIB] = [42; 1 * MIB];

#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Args {
    /// Port to listen on
    #[clap(short, long, default_value = "6969")]
    port: u16,
}

/// endless stream of bytes
async fn speedtest(_: Request<Body>) -> Result<Response<Body>> {
    log::debug!("starting stream");
    // let body_stream = stream::iter(chunk).cycle().map(|b| Ok::<_, Error>(b)); // endless stream
    let chunks = vec![&CHUNK];
    let body_stream = stream::iter(chunks)
        .cycle()
        .map(|b| Ok::<_, Error>(Bytes::from_static(b))); // endless stream
    Ok(Response::new(Body::wrap_stream(body_stream)))
}

async fn handle(req: Request<Body>) -> Result<Response<Body>> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") | (&Method::GET, "/speedtest") => speedtest(req).await,
        _ => Ok(Response::new(Body::from("hi"))),
    }
}

async fn shutdown_signal() {
    // Wait for the CTRL+C signal
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}

fn get_external_ip() -> Result<String> {
    use std::process::Command;
    let output = Command::new("dig")
        .args(["@resolver4.opendns.com", "myip.opendns.com", "+short"])
        .output()?;
    let output = std::str::from_utf8(&output.stdout)?.trim();
    Ok(output.to_string())
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Err(std::env::VarError::NotPresent) = std::env::var("RUST_LOG") {
        std::env::set_var("RUST_LOG", "warn,server=info")
    }

    pretty_env_logger::init();

    let opts = Args::parse();

    let external_ip = get_external_ip()?;
    log::info!("external ip: {}", external_ip);

    let addr = ([0, 0, 0, 0], opts.port).into();

    let make_svc = make_service_fn(|_conn| async { Ok::<_, Error>(service_fn(handle)) });

    let server = Server::bind(&addr)
        .serve(make_svc)
        .with_graceful_shutdown(shutdown_signal());

    log::info!("Listening on port {}", opts.port);
    log::info!(
        "make requests to http://{} or http://{}:{}",
        addr,
        external_ip,
        opts.port
    );

    server.await?;

    Ok(())
}
