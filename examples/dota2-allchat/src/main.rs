use std::fs::File;
use std::io::BufReader;

use haste::parser::{self, Context, Parser, Visitor};
use haste::valveprotos::dota2::{CdotaUserMsgChatMessage, EDotaUserMessages};
use haste::valveprotos::prost::Message;

struct MyVisitor;

impl Visitor for MyVisitor {
    fn on_packet(&mut self, _ctx: &Context, packet_type: u32, data: &[u8]) -> parser::Result<()> {
        if packet_type == EDotaUserMessages::DotaUmChatMessage as u32 {
            let msg = CdotaUserMsgChatMessage::decode(data)?;
            println!("{:?}", msg);
        }
        Ok(())
    }
}

fn main() -> parser::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let filepath = args.get(1);
    if filepath.is_none() {
        eprintln!("usage: dota2-allchat <filepath>");
        std::process::exit(42);
    }

    let file = File::open(filepath.unwrap())?;
    let buf_reader = BufReader::new(file);
    let mut parser = Parser::from_reader_with_visitor(buf_reader, MyVisitor)?;
    parser.run_to_end()
}
