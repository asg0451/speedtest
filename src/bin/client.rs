use anyhow::{anyhow, Result};
use hyper::{body::HttpBody as _, Client};
use std::env;
use tokio::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(std::env::VarError::NotPresent) = std::env::var("RUST_LOG") {
        std::env::set_var("RUST_LOG", "client=info")
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

async fn fetch_url(url: hyper::Uri) -> Result<()> {
    let client = Client::new();

    let mut res = client.get(url).await?;

    log::debug!("Response: {}", res.status());
    log::debug!("Headers: {:#?}\n", res.headers());

    // stream the response body, counting data and time
    // report on bytes/sec every now and then

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    let stream_task = tokio::task::spawn(async move {
        // every N chunks, send our count to the channel
        let chunk_group_size = 5_000;
        let mut bs = 0;
        let mut chunk_num = 0;
        let mut chunk_group_started_at = Instant::now();
        while let Some(next) = res.data().await {
            let chunk = next.unwrap();
            let size = chunk.len();
            bs += size;
            chunk_num += 1;

            if chunk_num > chunk_group_size {
                let now = Instant::now();

                tx.send((bs, now - chunk_group_started_at)).unwrap();
                bs = 0;
                chunk_num = 0;
                chunk_group_started_at = now;
            }
        }

        log::trace!("stream ends");
    });

    let report_task = tokio::spawn(async move {
        while let Some((bs, dur)) = rx.recv().await {
            let rate = bs as f64 / dur.as_secs_f64();
            log::info!("rate: {}", format_rate(rate));
        }
        log::trace!("report ends")
    });

    tokio::select! {
        _ = stream_task => (),
        _ = report_task => (),
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
