use std::fs::File;
use std::io::BufReader;

use anyhow::{Context as _, Result};
use haste::demofile::DemoFile;
use haste::parser::{Context, Parser, Visitor};
use haste::valveprotos::dota2::{CdotaUserMsgChatMessage, EDotaUserMessages};
use haste::valveprotos::prost::Message;

struct MyVisitor;

impl Visitor for MyVisitor {
    fn on_packet(&mut self, _ctx: &Context, packet_type: u32, data: &[u8]) -> Result<()> {
        if packet_type == EDotaUserMessages::DotaUmChatMessage as u32 {
            let msg = CdotaUserMsgChatMessage::decode(data)?;
            eprintln!("{:?}", msg);
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let filepath = args.get(1).context("usage: dota2-allchat <filepath>")?;
    let file = File::open(filepath)?;
    let buf_reader = BufReader::new(file);
    let demo_file = DemoFile::start_reading(buf_reader)?;
    let mut parser = Parser::from_stream_with_visitor(demo_file, MyVisitor)?;
    parser.run_to_end()
}
