use anyhow::Result;
use hyper::{body::HttpBody as _, Client};
use std::env;
use tokio::time::{Duration, Instant};

// A simple type alias so as to DRY.

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    // Some simple CLI args requirements...
    let url = match env::args().nth(1) {
        Some(url) => url,
        None => {
            println!("Usage: client <url>");
            return Ok(());
        }
    };

    // HTTPS requires picking a TLS implementation, so give a better
    // warning if the user tries to request an 'https' URL.
    let url = url.parse::<hyper::Uri>().unwrap();
    if url.scheme_str() != Some("http") {
        println!("This example only works with 'http' URLs.");
        return Ok(());
    }

    fetch_url(url).await
}

async fn fetch_url(url: hyper::Uri) -> Result<()> {
    let client = Client::new();

    let mut res = client.get(url).await?;

    println!("Response: {}", res.status());
    println!("Headers: {:#?}\n", res.headers());

    // stream the response body, counting data and time
    // report on bytes/sec every now and then

    let mut bs = 0;
    let mut last_reported_at = Instant::now();
    let ivl = Duration::from_secs(5);

    while let Some(next) = res.data().await {
        let chunk = next?;
        let size = chunk.len();
        bs += size;

        let now = Instant::now();
        let dur = now - last_reported_at;
        if dur > ivl {
            let rate = bs as f64 / dur.as_secs_f64();
            println!("rate: {}", format_rate(rate));
            last_reported_at = now;
            bs = 0;
        }
    }

    println!("end");
    Ok(())
}

fn format_rate(r: f64) -> String {
    let scales = [
        (1_000_000_000_000., "Tb/s"),
        (1_000_000_000., "Gb/s"),
        (1_000_000., "Mb/s"),
        (1_000., "Kb/s"),
        (1., "b/s"),
    ];
    for (factor, suffix) in scales {
        if r / factor > 1. {
            return format!("{:.1} {}", r / factor, suffix);
        }
    }
    return format!("{} b", r);
}
