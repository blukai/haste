use std::error::Error;
use std::io::{self, Read, Seek, SeekFrom};

use valveprotos::common as protos;

#[derive(Debug, Clone)]
pub struct CmdHeader {
    pub cmd: protos::EDemoCommands,
    pub body_compressed: bool,
    pub tick: i32,
    pub body_size: u32,
    // NOTE: it is siginficantly cheaper to sum n bytes that were read (cmd, tick body_size) then
    // to rely on Seek::stream_position.
    //
    /// size of the cmd header (/ how many bytes were read). can be used to unread the cmd header.
    pub size: u8,
}

pub trait CmdBody: Default + prost::Message {}

// Error
impl CmdBody for protos::CDemoStop {}
impl CmdBody for protos::CDemoFileHeader {}
impl CmdBody for protos::CDemoFileInfo {}
impl CmdBody for protos::CDemoSyncTick {}
impl CmdBody for protos::CDemoSendTables {}
impl CmdBody for protos::CDemoClassInfo {}
impl CmdBody for protos::CDemoStringTables {}
impl CmdBody for protos::CDemoPacket {}
// SignonPacket
impl CmdBody for protos::CDemoConsoleCmd {}
impl CmdBody for protos::CDemoCustomData {}
impl CmdBody for protos::CDemoCustomDataCallbacks {}
impl CmdBody for protos::CDemoUserCmd {}
impl CmdBody for protos::CDemoFullPacket {}
impl CmdBody for protos::CDemoSaveGame {}
impl CmdBody for protos::CDemoSpawnGroups {}
impl CmdBody for protos::CDemoAnimationData {}
impl CmdBody for protos::CDemoAnimationHeader {}
// Max
// IsCompressed

pub trait DemoStream<R: Read + Seek> {
    type ReadCmdHeaderError: Error + Send + Sync + 'static;
    type ReadCmdBodyError: Error + Send + Sync + 'static;

    // stream ops
    // ----

    fn seek(&mut self, pos: SeekFrom) -> Result<u64, io::Error>;

    fn stream_position(&mut self) -> Result<u64, io::Error>;

    /// reimplementation of nightly [`std::io::Seek::stream_len`].
    fn stream_len(&mut self) -> Result<u64, io::Error> {
        let old_pos = self.stream_position()?;
        let len = self.seek(SeekFrom::End(0))?;

        // avoid seeking a third time when we were already at the end of the stream. the branch is
        // usually way cheaper than a seek operation.
        if old_pos != len {
            self.seek(SeekFrom::Start(old_pos))?;
        }

        Ok(len)
    }

    fn is_eof(&mut self) -> Result<bool, io::Error> {
        Ok(self.stream_position()? == self.stream_len()?)
    }

    // cmd header
    // ----

    fn read_cmd_header(&mut self) -> Result<CmdHeader, Self::ReadCmdHeaderError>;

    fn unread_cmd_header(&mut self, cmd_header: &CmdHeader) -> Result<(), io::Error> {
        self.seek(SeekFrom::Current(-(cmd_header.size as i64)))
            .map(|_| ())
    }

    // cmd body
    // ----

    fn read_cmd_body(&mut self, cmd_header: &CmdHeader) -> Result<&[u8], Self::ReadCmdBodyError>;

    fn decode_cmd_body<T>(data: &[u8]) -> Result<T, Self::ReadCmdBodyError>
    where
        T: CmdBody;

    fn skip_cmd_body(&mut self, cmd_header: &CmdHeader) -> Result<(), io::Error> {
        self.seek(SeekFrom::Current(cmd_header.body_size as i64))
            .map(|_| ())
    }
}
