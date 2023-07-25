use crate::{protos::EDemoCommands, varint};
use std::io::{Read, Seek, SeekFrom};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // std
    #[error(transparent)]
    Io(#[from] std::io::Error),
    // 3rd party crates
    #[error(transparent)]
    Snap(#[from] snap::Error),
    #[error(transparent)]
    Prost(#[from] prost::DecodeError),
    // crate
    #[error(transparent)]
    Varint(#[from] varint::Error),
    // mod
    #[error("invalid header id (want {want:?}, got {got:?})")]
    InvalidHeader { want: [u8; 8], got: [u8; 8] },
    #[error("invalid cmd {0}")]
    InvalidCmd(u32),
}

pub type Result<T> = std::result::Result<T, Error>;

// strings in c/cpp are null terminated.
const DEMO_HEADER_ID: [u8; 8] = *b"PBDEMS2\0";

// NOTE: naming is based on stuff from demofile.h of valve's demoinfo2 thing.
#[derive(Default, Debug)]
pub struct DemoHeader {
    pub demofilestamp: [u8; 8],
    pub fileinfo_offset: i32,
    pub spawngroups_offset: i32,
}

#[derive(Debug)]
pub struct CmdHeader {
    pub command: EDemoCommands,
    pub is_compressed: bool,
    pub tick: u32,
    pub size: u32,
}

pub struct DemoFile<R: Read + Seek> {
    rdr: R,
}

impl<R: Read + Seek> DemoFile<R> {
    /// you should provide a reader that implements buffering (eg BufReader)
    /// because it'll be much more efficient.
    pub fn from_reader(rdr: R) -> Self {
        Self { rdr }
    }

    pub fn read_demo_header(&mut self) -> Result<DemoHeader> {
        let mut demo_header = DemoHeader::default();

        self.rdr.read_exact(&mut demo_header.demofilestamp)?;
        if demo_header.demofilestamp != DEMO_HEADER_ID {
            return Err(Error::InvalidHeader {
                want: DEMO_HEADER_ID,
                got: demo_header.demofilestamp,
            });
        }

        let mut buf = [0u8; 4];

        self.rdr.read_exact(&mut buf)?;
        demo_header.fileinfo_offset = i32::from_le_bytes(buf);

        self.rdr.read_exact(&mut buf)?;
        demo_header.spawngroups_offset = i32::from_le_bytes(buf);

        Ok(demo_header)
    }

    pub fn read_cmd_header(&mut self) -> Result<CmdHeader> {
        let (mut command, _) = varint::read_uvarint32(&mut self.rdr)?;
        const DEM_IS_COMPRESSED: u32 = EDemoCommands::DemIsCompressed as u32;
        let is_compressed = command & DEM_IS_COMPRESSED == DEM_IS_COMPRESSED;
        if is_compressed {
            command &= !DEM_IS_COMPRESSED;
        }
        let command = EDemoCommands::from_i32(command as i32).ok_or(Error::InvalidCmd(command))?;

        let (tick, _) = varint::read_uvarint32(&mut self.rdr)?;
        let (size, _) = varint::read_uvarint32(&mut self.rdr)?;

        Ok(CmdHeader {
            command,
            is_compressed,
            tick,
            size,
        })
    }

    // read_cmd reads bytes (cmd_header.size) from the reader into buf, if
    // compressed (cmd_header.is_compresed) it'll decompress the data, and
    // decode it into M.
    // NOTE: buf must be large enough to fit compressed and uncompressed data
    // simultaneously.
    pub fn read_cmd<M: prost::Message + Default>(
        &mut self,
        cmd_header: &CmdHeader,
        buf: &mut [u8],
    ) -> Result<M> {
        let (left, right) = buf.split_at_mut(cmd_header.size as usize);
        self.rdr.read_exact(left)?;

        let data = if cmd_header.is_compressed {
            let decompress_len = snap::raw::decompress_len(left)?;
            snap::raw::Decoder::new().decompress(left, right)?;
            // NOTE: we need to slice stuff up, because prost's decode can't
            // determine when to stop.
            &right[..decompress_len]
        } else {
            left
        };

        // TODO: prost does not suppoer allocator_api -> find a way to decode
        // protos with custom allocator
        M::decode(data).map_err(Error::Prost)
    }

    #[inline(always)]
    pub fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        self.rdr.seek(pos).map_err(Error::Io)
    }

    #[inline(always)]
    pub fn stream_position(&mut self) -> Result<u64> {
        self.rdr.stream_position().map_err(Error::Io)
    }
}
