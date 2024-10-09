use std::io::{self, Read, Seek};

use haste_core::demofile::DEMO_RECORD_BUFFER_SIZE;
use haste_core::demostream::{CmdHeader, DemoStream};
use valveprotos::common::{CDemoClassInfo, CDemoFullPacket, CDemoPacket, CDemoSendTables};

use crate::demostream::{
    decode_cmd_class_info, decode_cmd_packet, decode_cmd_send_tables, read_cmd_header,
    DecodeCmdError, ReadCmdHeaderError,
};

pub struct BroadcastFile<R: Read + Seek> {
    rdr: R,
    buf: Vec<u8>,
}

impl<R: Read + Seek> DemoStream for BroadcastFile<R> {
    type ReadCmdHeaderError = ReadCmdHeaderError;
    type ReadCmdError = io::Error;
    type DecodeCmdError = DecodeCmdError;

    // stream ops
    // ----

    fn seek(&mut self, pos: std::io::SeekFrom) -> Result<u64, io::Error> {
        self.rdr.seek(pos)
    }

    fn stream_position(&mut self) -> Result<u64, io::Error> {
        self.rdr.stream_position()
    }

    // cmd header
    // ----
    //
    // cmd headers are broadcasts are similar to demo file cmd headers, but encoding is different.
    //
    // thanks to saul for figuring it out. see
    // https://github.com/saul/demofile-net/blob/7d3d59e478dbd2b000f4efa2dac70ed1bf2e2b7f/src/DemoFile/HttpBroadcastReader.cs#L150

    fn read_cmd_header(
        &mut self,
    ) -> Result<haste_core::demostream::CmdHeader, Self::ReadCmdHeaderError> {
        read_cmd_header(&mut self.rdr)
    }

    // cmd
    // ----

    fn read_cmd(&mut self, cmd_header: &CmdHeader) -> Result<&[u8], Self::ReadCmdError> {
        let buf = &mut self.buf[..cmd_header.body_size as usize];
        self.rdr.read_exact(buf)?;
        Ok(buf)
    }

    #[inline(always)]
    fn decode_cmd_send_tables(data: &[u8]) -> Result<CDemoSendTables, Self::DecodeCmdError> {
        decode_cmd_send_tables(data)
    }

    #[inline(always)]
    fn decode_cmd_class_info(data: &[u8]) -> Result<CDemoClassInfo, Self::DecodeCmdError> {
        decode_cmd_class_info(data)
    }

    #[inline(always)]
    fn decode_cmd_packet(data: &[u8]) -> Result<CDemoPacket, Self::DecodeCmdError> {
        decode_cmd_packet(data)
    }

    #[inline(always)]
    fn decode_cmd_full_packet(_data: &[u8]) -> Result<CDemoFullPacket, Self::DecodeCmdError> {
        unimplemented!()
    }
}

impl<R: Read + Seek> BroadcastFile<R> {
    /// creates a new [`BroadcastFile`] instance from the given reader.
    ///
    /// # performance note
    ///
    /// for optimal performance make sure to provide a reader that implements buffering (for
    /// example [`std::io::BufReader`]).
    pub fn start_reading(rdr: R) -> Self {
        Self {
            rdr,
            buf: vec![0u8; DEMO_RECORD_BUFFER_SIZE],
        }
    }
}
