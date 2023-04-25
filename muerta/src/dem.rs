// fns for working with top-level dem data.

use crate::{
    error::{Error, Result},
    protos::EDemoCommands,
    varint::VarIntRead,
};
use std::io::Read;

const DEM_IS_COMPRESSED: u32 = EDemoCommands::DemIsCompressed as u32;

#[derive(thiserror::Error, Debug)]
pub enum DemError {
    #[error("unknown command {0}")]
    UnknownCommand(u32),
}

#[derive(Default, Debug)]
pub struct Header {
    pub demo_file_stamp: [u8; 8],
    pub file_info_offset: i32,
    pub unknown: [u8; 4],
}

impl Header {
    pub fn from_reader(rdr: &mut impl Read) -> Result<Self> {
        let mut header = Self::default();
        rdr.read_exact(&mut header.demo_file_stamp)?;

        let mut fio = [0u8; 4];
        rdr.read_exact(&mut fio)?;
        header.file_info_offset = i32::from_le_bytes(fio);

        rdr.read_exact(&mut header.unknown)?;

        Ok(header)
    }
}

#[derive(Debug)]
pub struct MsgHeader {
    pub command: EDemoCommands,
    pub is_compressed: bool,
    pub tick: u32,
    pub size: u32,
}

impl MsgHeader {
    pub fn from_reader(rdr: &mut impl Read) -> Result<Self> {
        let mut command = rdr.read_varu32()?;
        let is_compressed = command & DEM_IS_COMPRESSED == DEM_IS_COMPRESSED;
        if is_compressed {
            command &= !DEM_IS_COMPRESSED;
        }
        let command =
            EDemoCommands::from_i32(command as i32).ok_or(DemError::UnknownCommand(command))?;

        let tick = rdr.read_varu32()?;
        let size = rdr.read_varu32()?;

        Ok(MsgHeader {
            command,
            is_compressed,
            tick,
            size,
        })
    }
}

pub trait Msg<M: prost::Message + Default>: prost::Message {
    fn from_reader(rdr: &mut impl Read, msg_header: &MsgHeader, buf: &mut [u8]) -> Result<M>;
}

impl<M: prost::Message + Default> Msg<M> for M {
    // read_msg_data reads msg_header.size bytes from the reader into buf, if
    // compressed - decompresses, and M::decode s.
    fn from_reader(rdr: &mut impl Read, msg_header: &MsgHeader, buf: &mut [u8]) -> Result<Self> {
        let (left, right) = buf.split_at_mut(msg_header.size as usize);
        rdr.read_exact(left)?;

        let data = if msg_header.is_compressed {
            let decompress_len = snap::raw::decompress_len(left)?;
            snap::raw::Decoder::new().decompress(left, right)?;
            // NOTE: we need to slice stuff, because prost's decode does not
            // determine when to stop.
            &right[..decompress_len]
        } else {
            left
        };
        let data: &_ = data;

        // TODO: prost does not suppoer allocator_api -> find a way to decode
        // protos with custom allocator
        M::decode(data).map_err(Error::Decode)
    }
}

#[cfg(test)]
mod tests {
    use super::{Header, Msg, MsgHeader};
    use crate::protos;
    use expect_test::expect;
    use std::fs::File;

    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    #[test]
    fn dem_stuff() -> Result<()> {
        let mut file = File::open("../fixtures/7049305691_1283097277.dem")?;

        let header = Header::from_reader(&mut file)?;
        expect!["Header { demo_file_stamp: [80, 66, 68, 69, 77, 83, 50, 0], file_info_offset: 46809842, unknown: [125, 66, 202, 2] }"]
            .assert_eq(&format!("{:?}", &header));

        let msg_header = MsgHeader::from_reader(&mut file)?;
        expect!["MsgHeader { command: DemFileHeader, is_compressed: false, tick: 4294967295, size: 190 }"]
            .assert_eq(&format!("{:?}", &msg_header));

        let mut buf = vec![0u8; 1024 * 1024];
        let msg = protos::CDemoFileHeader::from_reader(&mut file, &msg_header, &mut buf[..]);
        expect![[r#"Ok(CDemoFileHeader { demo_file_stamp: "PBDEMS2\0", network_protocol: Some(47), server_name: Some("Valve Dota 2 USEast Server (srcds1007-iad1.121.200)"), client_name: Some("SourceTV Demo"), map_name: Some("start"), game_directory: Some("/opt/srcds/dota_custom/dota_v5634/dota"), fullpackets_version: Some(2), allow_clientside_entities: Some(true), allow_clientside_particles: Some(true), addons: Some(""), demo_version_name: Some("valve_demo_2"), demo_version_guid: Some("8e9d71ab-04a1-4c01-bb61-acfede27c046"), build_num: Some(9629), game: None })"#]]
            .assert_eq(&format!("{:?}", &msg));

        Ok(())
    }
}
