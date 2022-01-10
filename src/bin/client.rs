use anyhow::{anyhow, Result};
use hyper::{body::HttpBody as _, Client};
use std::env;
use tokio::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(std::env::VarError::NotPresent) = std::env::var("RUST_LOG") {
        std::env::set_var("RUST_LOG", "warn,client=info")
    }

    pretty_env_logger::init();

    let url = match env::args().nth(1) {
        Some(url) => url,
        None => {
            println!("Usage: client <url>");
            return Ok(());
        }
    };

    // http only
    let url = url.parse::<hyper::Uri>().unwrap();
    if url.scheme_str() != Some("http") {
        return Err(anyhow!("This example only works with 'http' URLs."));
    }

    fetch_url(url).await
}

#[derive(Debug)]
enum Msg {
    ByteCount(usize),
    ReportTick,
}

async fn fetch_url(url: hyper::Uri) -> Result<()> {
    let client = Client::new();

    let mut res = client.get(url).await?;

    log::debug!("Response: {}", res.status());
    log::debug!("Headers: {:#?}\n", res.headers());

    // task: response body -> channel
    // task: tick -> channel
    // task: channel -> count & report

    let (tx, mut rx) = tokio::sync::mpsc::channel(4096);

    let stream_task = {
        let tx = tx.clone();
        tokio::task::spawn(async move {
            // every N chunks, send our count to the channel
            while let Some(next) = res.data().await {
                let chunk = next.unwrap();
                let size = chunk.len();
                // TODO: buffer? maybe just inceasing the server's chunk size is enough?
                tx.send(Msg::ByteCount(size)).await.unwrap()
            }

            log::trace!("stream ends");
        })
    };

    let tick_task = {
        let tx = tx.clone();
        tokio::spawn(async move {
            use tokio::time::{self, Duration};
            let mut ivl = time::interval(Duration::from_secs(1));
            loop {
                // TODO: graceful shutdown, as an exercise, if i feel like it.
                ivl.tick().await;
                tx.send(Msg::ReportTick).await.unwrap()
            }
        })
    };

    let recv_task = tokio::spawn(async move {
        let mut last_reported_at = Instant::now();
        let mut byte_count = 0;
        while let Some(msg) = rx.recv().await {
            match msg {
                Msg::ByteCount(bs) => {
                    byte_count += bs;
                }
                Msg::ReportTick => {
                    let now = Instant::now();
                    let dur = now - last_reported_at;
                    let rate = byte_count as f64 / dur.as_secs_f64();
                    log::info!("rate: {}", format_rate(rate));
                    byte_count = 0;
                    last_reported_at = now;
                }
            }
        }
        log::trace!("report ends")
    });

    tokio::select! {
        _ = stream_task => (),
        _ = tick_task => (),
        _ = recv_task => (),
    }

    log::debug!("finished");
    Ok(())
}

fn format_rate(r: f64) -> String {
    let scales = [
        (1_000_000_000_000., "TiB/s", "Tb/s"),
        (1_000_000_000., "GiB/s", "Gb/s"),
        (1_000_000., "MiB/s", "Mb/s"),
        (1_000., "KiB/s", "Kb/s"),
        (1., "B/s", "b/s"),
    ];
    for (factor, suffix, non_si_suffix) in scales {
        if r / factor > 1. {
            return format!(
                "{:.1} {} ({:.1} {})",
                r / factor,
                suffix,
                (r / factor) * 8.,
                non_si_suffix
            );
        }
    }
    return format!("{} B", r);
}
