use std::fs::File;
use std::io::{self, BufReader};
use std::time::Duration;

use anyhow::{bail, Result};
use haste_broadcast::broadcastfile::BroadcastFile;
use haste_broadcast::broadcasthttp::BroadcastHttp;
use haste_core::demostream::CmdHeader;
use haste_core::parser::{Context, Parser, Visitor};

struct MyVisitor;

impl Visitor for MyVisitor {
    fn on_cmd(&mut self, _ctx: &Context, cmd_header: &CmdHeader, _data: &[u8]) -> Result<()> {
        eprintln!("{cmd_header:?}");
        Ok(())
    }
}

/// download broadcast
#[derive(argh::FromArgs)]
#[argh(subcommand, name = "download")]
struct DownloadCommand {
    /// broadcast url
    #[argh(option)]
    url: String,
    /// write broadcast to the given file;
    /// if not provided nothing will be written anywhere (can be ok to not provide when debugging
    /// stuff and the goal is to stare at logs or something)
    #[argh(option)]
    output: Option<String>,
}

impl DownloadCommand {
    async fn execute(self) -> Result<()> {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()?;
        let mut broadcast = BroadcastHttp::start_streaming(http_client, &self.url).await?;

        let mut file = if let Some(output) = self.output {
            Some(File::create(output)?)
        } else {
            None
        };

        loop {
            let mut packet = broadcast.prepare_packet().await?;
            if let Some(ref mut file) = file {
                io::copy(&mut packet, file)?;
            }
            // TODO: look for stop command or something, figure out how to stop gracefuly
        }
    }
}

/// parse broadcast
#[derive(argh::FromArgs)]
#[argh(subcommand, name = "parse")]
struct ParseCommand {
    /// broadcast url
    #[argh(option)]
    url: Option<String>,
    /// read broadcast from the given file
    #[argh(option)]
    filepath: Option<String>,
}

impl ParseCommand {
    async fn parse_from_url(url: &str) -> Result<()> {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()?;
        let broadcast = BroadcastHttp::start_streaming(http_client, url).await?;
        let mut parser = Parser::from_stream_with_visitor(broadcast, MyVisitor)?;
        parser.run_to_end()
    }

    fn parse_from_filepath(filepath: &str) -> Result<()> {
        let file = File::open(filepath)?;
        let buf_reader = BufReader::new(file);
        let demo_stream = BroadcastFile::start_reading(buf_reader);

        let mut parser = Parser::from_stream_with_visitor(demo_stream, MyVisitor)?;
        parser.run_to_end()
    }

    async fn execute(self) -> Result<()> {
        if let (Some(url), None) = (&self.url, &self.filepath) {
            return Self::parse_from_url(url).await;
        }

        if let (None, Some(filepath)) = (&self.url, &self.filepath) {
            return Self::parse_from_filepath(filepath);
        }

        bail!("invalid args; run {} help", env!("CARGO_PKG_NAME"));
    }
}

#[derive(argh::FromArgs)]
#[argh(subcommand)]
enum SubCommands {
    Download(DownloadCommand),
    Parse(ParseCommand),
}

impl SubCommands {
    async fn execute(self) -> Result<()> {
        match self {
            SubCommands::Download(download) => download.execute().await,
            SubCommands::Parse(parse) => parse.execute().await,
        }
    }
}

/// hanyanya fuwa
#[derive(argh::FromArgs)]
struct Args {
    #[argh(subcommand)]
    sub_command: SubCommands,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args = argh::from_env::<Args>();
    args.sub_command.execute().await
}
