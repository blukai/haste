use std::io::{self, BufRead};

use haste_core::demofile::DEMO_RECORD_BUFFER_SIZE;
use haste_core::demostream::{
    CmdHeader, DecodeCmdError, DemoStream, ReadCmdError, ReadCmdHeaderError,
};
use prost::Message;
use valveprotos::common::{
    CDemoClassInfo, CDemoFullPacket, CDemoPacket, CDemoSendTables, EDemoCommands,
};

pub struct Broadcast<R: BufRead> {
    rdr: R,
    buf: Vec<u8>,
}

impl<R: BufRead> DemoStream for Broadcast<R> {
    // stream ops
    // ----

    fn seek(&mut self, _pos: std::io::SeekFrom) -> Result<u64, io::Error> {
        unimplemented!()
    }

    fn stream_position(&mut self) -> Result<u64, io::Error> {
        unimplemented!()
    }

    fn is_at_eof(&mut self) -> Result<bool, io::Error> {
        Ok(self.rdr.fill_buf()?.is_empty())
    }

    // cmd header
    // ----
    //
    // cmd headers are broadcasts are similar to demo file cmd headers, but encoding is different.
    //
    // thanks to saul for figuring it out. see
    // https://github.com/saul/demofile-net/blob/7d3d59e478dbd2b000f4efa2dac70ed1bf2e2b7f/src/DemoFile/HttpBroadcastReader.cs#L150

    fn read_cmd_header(&mut self) -> Result<haste_core::demostream::CmdHeader, ReadCmdHeaderError> {
        // TODO: bytereader (bitreader-like) + migrate read_exact and similar instalces across the code
        // base to it (valve have CUtlBuffer for reference to make api similar).
        let mut buf = [0u8; size_of::<u32>()];

        let (cmd, cmd_n) = {
            self.rdr.read_exact(&mut buf[..1])?;
            let cmd = buf[0];
            (
                EDemoCommands::try_from(cmd as i32).map_err(|_| {
                    ReadCmdHeaderError::UnknownCmd {
                        raw: cmd as u32,
                        uncompressed: cmd as u32,
                    }
                })?,
                size_of::<u8>(),
            )
        };

        let (tick, tick_n) = {
            self.rdr.read_exact(&mut buf)?;
            (u32::from_le_bytes(buf) as i32, size_of::<u32>())
        };

        let (_unknown, unknown_n) = {
            self.rdr.read_exact(&mut buf[..1])?;
            (buf[0], size_of::<u8>())
        };

        let (body_size, body_size_n) = {
            self.rdr.read_exact(&mut buf)?;
            (u32::from_le_bytes(buf), size_of::<u32>())
        };

        Ok(CmdHeader {
            cmd,
            body_compressed: false,
            tick,
            body_size,
            size: (cmd_n + tick_n + body_size_n + unknown_n) as u8,
        })
    }

    fn unread_cmd_header(&mut self, _cmd_header: &CmdHeader) -> Result<(), io::Error> {
        unimplemented!()
    }

    // cmd
    // ----

    fn read_cmd(&mut self, cmd_header: &CmdHeader) -> Result<&[u8], ReadCmdError> {
        let buf = &mut self.buf[..cmd_header.body_size as usize];
        self.rdr.read_exact(buf)?;
        Ok(buf)
    }

    #[inline(always)]
    fn decode_cmd_send_tables(data: &[u8]) -> Result<CDemoSendTables, DecodeCmdError> {
        Ok(CDemoSendTables {
            // TODO: no-copy for send tables cmd
            // also think about how to do no-copy when decoding protobuf.
            data: Some((&data[4..]).to_vec()),
        })
    }

    #[inline(always)]
    fn decode_cmd_class_info(data: &[u8]) -> Result<CDemoClassInfo, DecodeCmdError> {
        CDemoClassInfo::decode(data).map_err(DecodeCmdError::DecodeError)
    }

    #[inline(always)]
    fn decode_cmd_packet(data: &[u8]) -> Result<CDemoPacket, DecodeCmdError> {
        Ok(CDemoPacket {
            // TODO: no-copy for packet cmd.
            // also think about how to do no-copy when decoding protobuf.
            data: Some(data.to_vec()),
        })
    }

    fn decode_cmd_full_packet(_data: &[u8]) -> Result<CDemoFullPacket, DecodeCmdError> {
        // NOTE: broadcasts don't seem to contain full packets
        unreachable!()
    }

    fn skip_cmd(&mut self, _cmd_header: &CmdHeader) -> Result<(), io::Error> {
        unimplemented!()
    }
}

impl<R: BufRead> Broadcast<R> {
    /// creates a new [`DemoFile`] instance from the given buf reader.
    pub fn start_reading(rdr: R) -> Self {
        Self {
            rdr,
            buf: vec![0u8; DEMO_RECORD_BUFFER_SIZE],
        }
    }
}
