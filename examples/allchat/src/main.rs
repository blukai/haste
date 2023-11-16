use haste_dota2::{
    haste_dota2_protos::{self, prost::Message},
    parser::{self, Parser, Visitor},
};
use std::{fs::File, io::BufReader};

struct MyVisitor;

impl Visitor for MyVisitor {
    fn visit_packet(&self, packet_type: u32, data: &[u8]) -> parser::Result<()> {
        if packet_type == haste_dota2_protos::EDotaUserMessages::DotaUmChatMessage as u32 {
            let msg = haste_dota2_protos::CdotaUserMsgChatMessage::decode(data)?;
            println!("{:?}", msg);
        }
        Ok(())
    }
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let filepath = args.get(1);
    if filepath.is_none() {
        eprintln!("usage: allchat <filepath>");
        std::process::exit(42);
    }

    let file = File::open(filepath.unwrap())?;
    let buf_reader = BufReader::new(file);
    let mut parser = Parser::from_reader(buf_reader, MyVisitor)?;
    parser.parse_all()
}
