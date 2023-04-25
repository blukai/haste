// fns for working with packets.

use crate::{
    bitreader::BitReader,
    error::{Error, Result},
};

#[derive(Debug)]
pub struct Header {
    pub command: u32,
    pub size: u32,
}

impl Header {
    pub fn from_bitreader(br: &mut BitReader) -> Result<Self> {
        let command = br.read_ubitvar()?;
        let size = br.read_varu32()?;
        Ok(Self { command, size })
    }
}

pub trait Msg<M: prost::Message + Default>: prost::Message {
    fn from_bitreader(br: &mut BitReader, packet_header: &Header, buf: &mut [u8]) -> Result<M>;
}

impl<M: prost::Message + Default> Msg<M> for M {
    fn from_bitreader(br: &mut BitReader, packet_header: &Header, buf: &mut [u8]) -> Result<M> {
        let dst = &mut buf[..packet_header.size as usize];
        br.read_bytes(dst)?;
        let data: &_ = dst;
        M::decode(data).map_err(Error::Decode)
    }
}

#[cfg(test)]
mod tests {
    use super::{Header, Msg as PacketMsg};
    use crate::{
        bitreader::BitReader,
        dem::{self, Msg as DemMsg},
        protos,
    };
    use expect_test::expect;
    use std::{
        fs::File,
        io::{Seek, SeekFrom},
    };

    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    #[test]
    fn packet_stuff() -> Result<()> {
        let mut file = File::open("../fixtures/7049305691_1283097277.dem")?;
        dem::Header::from_reader(&mut file)?;
        loop {
            let msg_header = dem::MsgHeader::from_reader(&mut file)?;
            match msg_header.command {
                protos::EDemoCommands::DemSignonPacket | protos::EDemoCommands::DemPacket => {
                    let mut buf = vec![0u8; 1024 * 1024];
                    let proto =
                        protos::CDemoPacket::from_reader(&mut file, &msg_header, &mut buf[..])?;
                    let mut br = BitReader::new(&proto.data.as_ref().unwrap());

                    let packet_header = Header::from_bitreader(&mut br)?;
                    expect!["Header { command: 4, size: 5 }"]
                        .assert_eq(&format!("{:?}", &packet_header));

                    match packet_header.command {
                        c if c == protos::NetMessages::NetTick as u32 => {
                            let proto = protos::CnetMsgTick::from_bitreader(
                                &mut br,
                                &packet_header,
                                &mut buf,
                            )?;
                            expect!["CnetMsgTick { tick: Some(176), host_frametime: None, host_frametime_std_deviation: None, host_computationtime: None, host_computationtime_std_deviation: None, host_framestarttime_std_deviation: None, host_loss: Some(0), host_unfiltered_frametime: None, hltv_replay_flags: None }"]
                                .assert_eq(&format!("{:?}", &proto));

                            return Ok(());
                        }
                        _ => {
                            br.skip_bytes(packet_header.size as u64)?;
                        }
                    }
                }
                _ => {
                    file.seek(SeekFrom::Current(msg_header.size as i64))?;
                }
            }
        }
    }
}
