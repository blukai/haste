use std::io::{Read, SeekFrom};

use haste_core::demostream::{CmdHeader, DecodeCmdError, DemoStream, ReadCmdHeaderError};
use prost::Message;
use valveprotos::common::{
    CDemoClassInfo, CDemoFullPacket, CDemoPacket, CDemoSendTables, EDemoCommands,
};

// cmd header
// ----
//
// cmd headers are broadcasts are similar to demo file cmd headers, but encoding is different.
//
// thanks to saul for figuring it out. see
// https://github.com/saul/demofile-net/blob/7d3d59e478dbd2b000f4efa2dac70ed1bf2e2b7f/src/DemoFile/HttpBroadcastReader.cs#L150

#[inline(always)]
pub(crate) fn read_cmd_header<R: Read>(mut rdr: R) -> Result<CmdHeader, ReadCmdHeaderError> {
    // TODO: bytereader (bitreader-like) + migrate read_exact and similar instalces across the code
    // base to it (valve have CUtlBuffer for reference to make api similar).
    let mut buf = [0u8; size_of::<u32>()];

    let (cmd, cmd_n) = {
        rdr.read_exact(&mut buf[..1])?;
        let cmd = buf[0];
        (
            EDemoCommands::try_from(cmd as i32).map_err(|_| ReadCmdHeaderError::UnknownCmd {
                raw: cmd as u32,
                uncompressed: cmd as u32,
            })?,
            size_of::<u8>(),
        )
    };

    let (tick, tick_n) = {
        rdr.read_exact(&mut buf)?;
        (u32::from_le_bytes(buf) as i32, size_of::<u32>())
    };

    let (_unknown, unknown_n) = {
        rdr.read_exact(&mut buf[..1])?;
        (buf[0], size_of::<u8>())
    };

    let (body_size, body_size_n) = {
        rdr.read_exact(&mut buf)?;
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

// cmd
// ----

#[inline(always)]
pub(crate) fn decode_cmd_send_tables(data: &[u8]) -> Result<CDemoSendTables, DecodeCmdError> {
    Ok(CDemoSendTables {
        // TODO: no-copy for send tables cmd
        // also think about how to do no-copy when decoding protobuf.
        data: Some((&data[4..]).to_vec()),
    })
}

#[inline(always)]
pub(crate) fn decode_cmd_class_info(data: &[u8]) -> Result<CDemoClassInfo, DecodeCmdError> {
    CDemoClassInfo::decode(data).map_err(DecodeCmdError::DecodeProtobufError)
}

#[inline(always)]
pub(crate) fn decode_cmd_packet(data: &[u8]) -> Result<CDemoPacket, DecodeCmdError> {
    Ok(CDemoPacket {
        // TODO: no-copy for packet cmd.
        // also think about how to do no-copy when decoding protobuf.
        data: Some(data.to_vec()),
    })
}

#[inline(always)]
pub(crate) fn decode_cmd_full_packet(_data: &[u8]) -> Result<CDemoFullPacket, DecodeCmdError> {
    // NOTE: broadcasts don't seem to contain full packets
    unreachable!()
}

// other
// ----

pub(crate) fn scan_for_last_tick(demo_stream: &mut impl DemoStream) -> Result<i32, anyhow::Error> {
    let mut last_tick: i32 = -1;
    let backup = demo_stream.stream_position()?;
    loop {
        match demo_stream.read_cmd_header() {
            Ok(cmd_header) => {
                last_tick = cmd_header.tick;
                demo_stream.skip_cmd(&cmd_header)?;
            }
            Err(_) if demo_stream.is_at_eof().unwrap_or_default() => {
                demo_stream.seek(SeekFrom::Start(backup))?;
                return Ok(last_tick);
            }
            Err(err) => return Err(err.into()),
        }
    }
}
