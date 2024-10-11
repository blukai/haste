use std::fs::File;
use std::io::{BufReader, Write};
use std::time::Duration;

use anyhow::{bail, Result};
use haste::broadcast::{BroadcastFile, BroadcastHttp};
use haste::demostream::CmdHeader;
use haste::parser::{Context, Parser, Visitor};

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
    #[argh(option)]
    output: String,
}

impl DownloadCommand {
    async fn execute(self) -> Result<()> {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()?;
        let mut broadcast = BroadcastHttp::start_streaming(http_client, &self.url).await?;
        let mut file = File::create(&self.output)?;
        while let Some(packet) = broadcast.next_packet().await {
            // TODO: graceful stop? atm this will error out on "eof".
            file.write_all(packet?.as_ref())?;
        }
        Ok(())
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

        let demo_stream = BroadcastHttp::start_streaming(http_client, url).await?;
        let mut parser = Parser::from_stream_with_visitor(demo_stream, MyVisitor)?;

        loop {
            // you have to "drive" broadcast manualy in order to parse it
            let demo_stream = parser.demo_stream_mut();
            match demo_stream.next_packet().await {
                Some(Ok(_)) => {
                    // run_to_end does not mean to end of the replay, but rather to end of current
                    // data; it is okay to run it again when you know that more data was supplied,
                    // it'll continue from where it stopped at.
                    parser.run_to_end()?;
                }
                Some(Err(err)) => return Err(err.into()),
                None => return Ok(()),
            }
        }
    }

    fn parse_from_filepath(filepath: &str) -> Result<()> {
        let file = File::open(filepath)?;
        let buf_reader = BufReader::new(file);
        let broadcast_file = BroadcastFile::start_reading(buf_reader);
        let mut parser = Parser::from_stream_with_visitor(broadcast_file, MyVisitor)?;
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
