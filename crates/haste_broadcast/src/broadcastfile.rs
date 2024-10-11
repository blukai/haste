use std::io::{self, Read, Seek, SeekFrom};

use haste_core::demofile::DEMO_RECORD_BUFFER_SIZE;
use haste_core::demostream::{
    CmdHeader, DecodeCmdError, DemoStream, ReadCmdError, ReadCmdHeaderError,
};
use valveprotos::common::{CDemoClassInfo, CDemoFullPacket, CDemoPacket, CDemoSendTables};

use crate::demostream::{
    decode_cmd_class_info, decode_cmd_full_packet, decode_cmd_packet, decode_cmd_send_tables,
    read_cmd_header, scan_for_last_tick,
};

/// allows to read recorded broadcasts.
///
/// the format is:
/// - `/0/start`
/// - `/<signup_fragment>/full`
/// - `/<signup_fragment>/delta`
/// - `/<signup_fragment + 1>/delta`
/// - and so on...
pub struct BroadcastFile<R: Read + Seek> {
    rdr: R,
    buf: Vec<u8>,
    total_ticks: Option<i32>,
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
            total_ticks: None,
        }
    }
}

impl<R: Read + Seek> DemoStream for BroadcastFile<R> {
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
        read_cmd_header(&mut self.rdr)
    }

    // cmd body
    // ----

    fn read_cmd(&mut self, cmd_header: &CmdHeader) -> Result<&[u8], ReadCmdError> {
        assert!(!cmd_header.body_compressed);

        let data = &mut self.buf[..cmd_header.body_size as usize];
        self.rdr.read_exact(data)?;
        Ok(data)
    }

    #[inline(always)]
    fn decode_cmd_send_tables(data: &[u8]) -> Result<CDemoSendTables, DecodeCmdError> {
        decode_cmd_send_tables(data)
    }

    #[inline(always)]
    fn decode_cmd_class_info(data: &[u8]) -> Result<CDemoClassInfo, DecodeCmdError> {
        decode_cmd_class_info(data)
    }

    #[inline(always)]
    fn decode_cmd_packet(data: &[u8]) -> Result<CDemoPacket, DecodeCmdError> {
        decode_cmd_packet(data)
    }

    #[inline(always)]
    fn decode_cmd_full_packet(data: &[u8]) -> Result<CDemoFullPacket, DecodeCmdError> {
        decode_cmd_full_packet(data)
    }

    // other
    // ----

    fn start_position(&self) -> u64 {
        0
    }

    fn total_ticks(&mut self) -> Result<i32, anyhow::Error> {
        if self.total_ticks.is_none() {
            self.total_ticks = Some(scan_for_last_tick(self)?);
        }
        Ok(self.total_ticks.unwrap())
    }
}
