//! NOTE: this is not really an example, this is very work in progress experiment for tv
//! broadcasts.
use std::fs::File;
use std::io::BufReader;

use anyhow::{Context, Result};
use haste::bitreader::BitReader;
use haste::tvbroadcast::TvBroadcast;
use haste::valveprotos::common::{CDemoPacket, EDemoCommands};

fn handle_cmd_packet(cmd: CDemoPacket) -> Result<()> {
    let mut buf = vec![0u8; 10 << 20];

    let data = cmd.data.unwrap_or_default();
    let mut br = BitReader::new(&data);

    while br.num_bits_left() > 8 {
        let command = br.read_ubitvar();
        let size = br.read_uvarint32() as usize;

        let buf = &mut buf[..size];
        br.read_bytes(buf);

        match command {
            _ => {
                eprintln!("unhandled packet command {}", command);
            }
        }
    }

    br.is_overflowed()?;
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let filepath = args.get(1).context("usage: tv <filepath>")?;

    let file = File::open(filepath)?;
    let buf_reader = BufReader::new(file);

    let mut tv_broadcast = TvBroadcast::from_reader(buf_reader);
    loop {
        match tv_broadcast.read_cmd_header() {
            Ok(cmd_header) => {
                eprintln!("{:?}", &cmd_header);
                let cmd_body = tv_broadcast.read_cmd_body(&cmd_header)?;
                match cmd_header.cmd {
                    EDemoCommands::DemPacket => {
                        handle_cmd_packet(CDemoPacket {
                            data: Some(cmd_body.to_vec()),
                        })?;
                    }
                    _ => {}
                }
            }
            Err(err) => {
                if tv_broadcast.is_eof().unwrap_or_default() {
                    return Ok(());
                }
                return Err(err.into());
            }
        }
    }
}
