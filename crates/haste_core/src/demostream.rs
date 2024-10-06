use std::any::type_name;
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

// TODO: proper name for non-protobuf decoder error
#[derive(thiserror::Error, Debug)]
pub enum CmdBodyDecodeAltError {
    #[error(transparent)]
    DecodeProtobufError(#[from] prost::DecodeError),
}

pub trait CmdBody: Default + prost::Message {
    #[inline(always)]
    fn decode_protobuf(data: &[u8]) -> Result<Self, prost::DecodeError> {
        Self::decode(data)
    }

    // TODO: proper name for non-protobuf decoder
    fn decode_alt(_data: &[u8]) -> Result<Self, CmdBodyDecodeAltError> {
        unimplemented!("TODO: impl alt decoder for {}", type_name::<Self>())
    }
}

// Error
impl CmdBody for protos::CDemoStop {}
impl CmdBody for protos::CDemoFileHeader {}
impl CmdBody for protos::CDemoFileInfo {}
impl CmdBody for protos::CDemoSyncTick {}

impl CmdBody for protos::CDemoSendTables {
    fn decode_alt(data: &[u8]) -> Result<Self, CmdBodyDecodeAltError> {
        Ok(protos::CDemoSendTables {
            // TODO: no-copy for send tables cmd
            // also think about how to do no-copy when decoding protobuf.
            data: Some((&data[4..]).to_vec()),
        })
    }
}

impl CmdBody for protos::CDemoClassInfo {
    fn decode_alt(data: &[u8]) -> Result<Self, CmdBodyDecodeAltError> {
        Self::decode_protobuf(data).map_err(CmdBodyDecodeAltError::DecodeProtobufError)
    }
}

impl CmdBody for protos::CDemoStringTables {}

impl CmdBody for protos::CDemoPacket {
    fn decode_alt(data: &[u8]) -> Result<Self, CmdBodyDecodeAltError> {
        Ok(protos::CDemoPacket {
            // TODO: no-copy for packet cmd.
            // also think about how to do no-copy when decoding protobuf.
            data: Some(data.to_vec()),
        })
    }
}

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
