use std::{fs::File, io::Write, time::Duration};

use anyhow::Result;
use haste_broadcast::httpbroadcast::{default_headers, HttpBroadcast};

#[derive(argh::FromArgs)]
/// Hanyanya fuwa
struct Args {
    /// broadcast url
    #[argh(option)]
    url: String,
    /// write broadcast to the given file
    #[argh(option)]
    output: Option<String>,
    /// unexplained explanation
    #[argh(option)]
    app_id: Option<u32>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    env_logger::init();

    let args = argh::from_env::<Args>();

    let mut http_client_builder = reqwest::Client::builder().timeout(Duration::from_secs(3));
    if let Some(app_id) = args.app_id {
        http_client_builder = http_client_builder.default_headers(default_headers(app_id)?);
    }
    let http_client = http_client_builder.build()?;

    let mut http_broadcast = HttpBroadcast::start_streaming(http_client, args.url).await?;

    let mut file = if let Some(output) = args.output {
        Some(File::create(output)?)
    } else {
        None
    };

    loop {
        let packet = http_broadcast.next_packet().await?;
        if let Some(ref mut file) = file {
            file.write_all(packet.as_ref())?;
        }
        // TODO: look for stop command or something, figure out how to stop gracefuly
    }
}
