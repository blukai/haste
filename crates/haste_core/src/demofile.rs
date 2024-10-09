use std::io::{self, Read, Seek, SeekFrom};

use dungers::varint;
use prost::Message;
use valveprotos::common::{
    CDemoClassInfo, CDemoFullPacket, CDemoPacket, CDemoSendTables, EDemoCommands,
};
use valveprotos::prost;

use crate::demostream::{CmdHeader, DemoStream};

// #define DEMO_RECORD_BUFFER_SIZE 2*1024*1024
//
// NOTE: read_cmd reads bytes (cmd_header.body_size) from the rdr into buf, if cmd is compressed
// (cmd_header.body_compressed) it'll decompress the data. buf must be large enough to fit
// compressed and uncompressed data simultaneously.
pub const DEMO_RECORD_BUFFER_SIZE: usize = 2 * 1024 * 1024;

// #define DEMO_HEADER_ID "HL2DEMO"
//
// NOTE: strings in c/cpp are null terminated.
const DEMO_HEADER_ID_SIZE: usize = 8;
const DEMO_HEADER_ID: [u8; DEMO_HEADER_ID_SIZE] = *b"PBDEMS2\0";

// NOTE: naming is based on stuff from demofile.h of valve's demoinfo2 thing.
#[derive(Debug, Clone)]
pub struct DemoHeader {
    pub demofilestamp: [u8; DEMO_HEADER_ID_SIZE],
    pub fileinfo_offset: i32,
    pub spawngroups_offset: i32,
}

#[derive(thiserror::Error, Debug)]
pub enum ReadDemoHeaderError {
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error("invalid demo file stamp (got {got:?}; want id {DEMO_HEADER_ID:?})")]
    InvalidDemoFileStamp { got: [u8; DEMO_HEADER_ID_SIZE] },
}

pub fn read_demo_header<R: Read>(mut rdr: R) -> Result<DemoHeader, ReadDemoHeaderError> {
    let mut demofilestamp = [0u8; DEMO_HEADER_ID_SIZE];
    rdr.read_exact(&mut demofilestamp)?;
    if demofilestamp != DEMO_HEADER_ID {
        return Err(ReadDemoHeaderError::InvalidDemoFileStamp { got: demofilestamp });
    }

    let mut buf = [0u8; size_of::<i32>()];

    rdr.read_exact(&mut buf)?;
    let fileinfo_offset = i32::from_le_bytes(buf);

    rdr.read_exact(&mut buf)?;
    let spawngroups_offset = i32::from_le_bytes(buf);

    Ok(DemoHeader {
        demofilestamp,
        fileinfo_offset,
        spawngroups_offset,
    })
}

#[derive(thiserror::Error, Debug)]
pub enum ReadCmdHeaderError {
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error(transparent)]
    ReadVarintError(#[from] varint::ReadVarintError),
    #[error("unknown cmd (raw {raw}; uncompressed {uncompressed})")]
    UnknownCmd { raw: u32, uncompressed: u32 },
}

#[derive(thiserror::Error, Debug)]
pub enum ReadCmdError {
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error(transparent)]
    DecompressError(#[from] snap::Error),
}

#[derive(Debug)]
pub struct DemoFile<R: Read + Seek> {
    rdr: R,
    buf: Vec<u8>,
    demo_header: DemoHeader,
}

impl<R: Read + Seek> DemoStream for DemoFile<R> {
    type ReadCmdHeaderError = ReadCmdHeaderError;
    type ReadCmdError = ReadCmdError;
    type DecodeCmdError = prost::DecodeError;

    // stream ops
    // ----

    /// delegated from [`std::io::Seek`].
    #[inline]
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, io::Error> {
        self.rdr.seek(pos)
    }

    /// delegated from [`std::io::Seek`].
    ///
    /// # note
    ///
    /// be aware that this method can be quite expensive. it might be best to make sure not to call
    /// it too frequently.
    #[inline]
    fn stream_position(&mut self) -> Result<u64, io::Error> {
        self.rdr.stream_position()
    }

    // cmd header
    // ----

    fn read_cmd_header(&mut self) -> Result<CmdHeader, ReadCmdHeaderError> {
        let (cmd, cmd_n, body_compressed) = {
            let (cmd_raw, n) = varint::read_uvarint32(&mut self.rdr)?;

            const DEM_IS_COMPRESSED: u32 = EDemoCommands::DemIsCompressed as u32;
            let body_compressed = cmd_raw & DEM_IS_COMPRESSED == DEM_IS_COMPRESSED;

            let cmd = if body_compressed {
                cmd_raw & !DEM_IS_COMPRESSED
            } else {
                cmd_raw
            };

            (
                EDemoCommands::try_from(cmd as i32).map_err(|_| {
                    ReadCmdHeaderError::UnknownCmd {
                        raw: cmd_raw,
                        uncompressed: cmd,
                    }
                })?,
                n,
                body_compressed,
            )
        };

        let (tick, tick_n) = {
            let (tick, n) = varint::read_uvarint32(&mut self.rdr)?;
            // NOTE: tick is set to u32::MAX before before all pre-game initialization messages are
            // sent.
            // ticks everywhere are represented as i32, casting u32::MAX to i32 is okay because
            // bits in u32::MAX == bits in -1 i32.
            let tick = tick as i32;
            (tick, n)
        };

        let (body_size, body_size_n) = varint::read_uvarint32(&mut self.rdr)?;

        Ok(CmdHeader {
            cmd,
            body_compressed,
            tick,
            body_size,
            size: (cmd_n + tick_n + body_size_n) as u8,
        })
    }

    // cmd body
    // ----

    fn read_cmd(&mut self, cmd_header: &CmdHeader) -> Result<&[u8], ReadCmdError> {
        let (left, right) = self.buf.split_at_mut(cmd_header.body_size as usize);
        self.rdr.read_exact(left)?;

        if cmd_header.body_compressed {
            let decompress_len = snap::raw::decompress_len(left)?;
            snap::raw::Decoder::new().decompress(left, right)?;
            // NOTE: we need to slice stuff up, because prost's decode can't
            // determine when to stop.
            Ok(&right[..decompress_len])
        } else {
            Ok(left)
        }
    }

    #[inline(always)]
    fn decode_cmd_send_tables(data: &[u8]) -> Result<CDemoSendTables, Self::DecodeCmdError> {
        CDemoSendTables::decode(data)
    }

    #[inline(always)]
    fn decode_cmd_class_info(data: &[u8]) -> Result<CDemoClassInfo, Self::DecodeCmdError> {
        CDemoClassInfo::decode(data)
    }

    #[inline(always)]
    fn decode_cmd_packet(data: &[u8]) -> Result<CDemoPacket, Self::DecodeCmdError> {
        CDemoPacket::decode(data)
    }

    #[inline(always)]
    fn decode_cmd_full_packet(data: &[u8]) -> Result<CDemoFullPacket, Self::DecodeCmdError> {
        CDemoFullPacket::decode(data)
    }
}

impl<R: Read + Seek> DemoFile<R> {
    /// creates a new [`DemoFile`] instance from the given reader.
    ///
    /// # performance note
    ///
    /// for optimal performance make sure to provide a reader that implements buffering (for
    /// example [`std::io::BufReader`]).
    pub fn start_reading(mut rdr: R) -> Result<Self, ReadDemoHeaderError> {
        let demo_header = read_demo_header(&mut rdr)?;
        Ok(Self {
            rdr,
            buf: vec![0u8; DEMO_RECORD_BUFFER_SIZE],
            demo_header,
        })
    }

    pub fn demo_header(&self) -> &DemoHeader {
        &self.demo_header
    }
}
