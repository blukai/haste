use std::error::Error;
use std::io::{self, SeekFrom};

use valveprotos::common::{
    CDemoClassInfo, CDemoFullPacket, CDemoPacket, CDemoSendTables, EDemoCommands,
};

#[derive(Debug, Clone)]
pub struct CmdHeader {
    pub cmd: EDemoCommands,
    pub body_compressed: bool,
    pub tick: i32,
    pub body_size: u32,
    // NOTE: it is siginficantly cheaper to sum n bytes that were read (cmd, tick body_size) then
    // to rely on Seek::stream_position.
    //
    /// size of the cmd header (/ how many bytes were read). can be used to unread the cmd header.
    pub size: u8,
}

pub trait DemoStream {
    type ReadCmdHeaderError: Error + Send + Sync + 'static;
    type ReadCmdError: Error + Send + Sync + 'static;
    type DecodeCmdError: Error + Send + Sync + 'static;

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

    // cmd
    // ----

    fn read_cmd(&mut self, cmd_header: &CmdHeader) -> Result<&[u8], Self::ReadCmdError>;

    // TODO: should DemoStream require decoders for all cmds to be implemented?
    //
    // Error (no msg)
    // Stop (empty msg)
    // fn decode_cmd_file_header(data: &[u8]) -> Result<CDemoFileHeader, Self::DecodeCmdError>;
    // fn decode_cmd_file_info(data: &[u8]) -> Result<CDemoFileInfo, Self::DecodeCmdError>;
    // SyncTick (empty msg)
    fn decode_cmd_send_tables(data: &[u8]) -> Result<CDemoSendTables, Self::DecodeCmdError>;
    fn decode_cmd_class_info(data: &[u8]) -> Result<CDemoClassInfo, Self::DecodeCmdError>;
    // fn decode_cmd_string_tables(data: &[u8]) -> Result<CDemoStringTables, Self::DecodeCmdError>;
    fn decode_cmd_packet(data: &[u8]) -> Result<CDemoPacket, Self::DecodeCmdError>;
    // SignonPacket (same as Packet)
    // fn decode_cmd_console_cmd(data: &[u8]) -> Result<CDemoConsoleCmd, Self::DecodeCmdError>;
    // fn decode_cmd_custom_data(data: &[u8]) -> Result<CDemoCustomData, Self::DecodeCmdError>;
    // fn decode_cmd_custom_data_callbacks(data: &[u8]) -> Result<CDemoCustomDataCallbacks, Self::DecodeCmdError>;
    // fn decode_cmd_user_cmd(data: &[u8]) -> Result<CDemoUserCmd, Self::DecodeCmdError>;
    fn decode_cmd_full_packet(data: &[u8]) -> Result<CDemoFullPacket, Self::DecodeCmdError>;
    // fn decode_cmd_save_game(data: &[u8]) -> Result<CDemoSaveGame, Self::DecodeCmdError>;
    // fn decode_cmd_spawn_groups(data: &[u8]) -> Result<CDemoSpawnGroups, Self::DecodeCmdError>;
    // fn decode_cmd_animation_data(data: &[u8]) -> Result<CDemoAnimationData, Self::DecodeCmdError>;
    // fn decode_cmd_animation_header(data: &[u8]) -> Result<CDemoAnimationHeader, Self::DecodeCmdError>;
    // Max
    // IsCompressed (flag)

    fn skip_cmd(&mut self, cmd_header: &CmdHeader) -> Result<(), io::Error> {
        self.seek(SeekFrom::Current(cmd_header.body_size as i64))
            .map(|_| ())
    }
}
