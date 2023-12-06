use crate::{
    haste_dota2_protos::{
        prost::{self, Message},
        CDemoFileInfo, EDemoCommands,
    },
    varint,
};
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
    #[error("unexpected header id (want {want:?}, got {got:?})")]
    UnexpectedHeaderId { want: [u8; 8], got: [u8; 8] },
    #[error("unknown cmd {0}")]
    UnknownCmd(u32),
    #[error("expected cmd (cmd {0:?}")]
    ExpectedCmd(EDemoCommands),
}

pub type Result<T> = std::result::Result<T, Error>;

// strings in c/cpp are null terminated.
const DEMO_HEADER_ID: [u8; 8] = *b"PBDEMS2\0";

// NOTE: naming is based on stuff from demofile.h of valve's demoinfo2 thing.
#[derive(Debug, Default)]
pub struct DemoHeader {
    pub demofilestamp: [u8; 8],
    pub fileinfo_offset: i32,
    pub spawngroups_offset: i32,
}

#[derive(Debug)]
pub struct CmdHeader {
    pub command: EDemoCommands,
    pub is_compressed: bool,
    pub tick: i32,
    pub size: u32,
    // offset is how many bytes were read. offset can be used to do a backup
    // (/unread cmd header) - offset is cheaper then calling stream_position
    // method before reading cmd header.
    pub offset: usize,
}

#[derive(Debug)]
pub struct DemoFile<R: Read + Seek> {
    rdr: R,
    demo_header: Option<DemoHeader>,
    file_info: Option<CDemoFileInfo>,
}

impl<R: Read + Seek> DemoFile<R> {
    /// you should provide a reader that implements buffering (eg BufReader)
    /// because it'll be much more efficient.
    pub fn from_reader(rdr: R) -> Self {
        Self {
            rdr,
            demo_header: None,
            file_info: None,
        }
    }

    // demoheader_t * 	ReadDemoHeader( CDemoPlaybackParameters_t const *pPlaybackParameters );
    pub fn read_demo_header(&mut self) -> Result<&DemoHeader> {
        let mut demo_header = DemoHeader::default();

        self.rdr.read_exact(&mut demo_header.demofilestamp)?;
        if demo_header.demofilestamp != DEMO_HEADER_ID {
            return Err(Error::UnexpectedHeaderId {
                want: DEMO_HEADER_ID,
                got: demo_header.demofilestamp,
            });
        }

        let mut buf = [0u8; 4];

        self.rdr.read_exact(&mut buf)?;
        demo_header.fileinfo_offset = i32::from_le_bytes(buf);

        self.rdr.read_exact(&mut buf)?;
        demo_header.spawngroups_offset = i32::from_le_bytes(buf);

        self.demo_header = Some(demo_header);
        Ok(unsafe { self.demo_header.as_ref().unwrap_unchecked() })
    }

    // void ReadCmdHeader( unsigned char& cmd, int& tick, int &nPlayerSlot );
    pub fn read_cmd_header(&mut self) -> Result<CmdHeader> {
        debug_assert!(
            self.demo_header.is_some(),
            "expected demo header to have been read"
        );

        let (command, command_ot, is_compressed) = {
            let (c, ot) = varint::read_uvarint32(&mut self.rdr)?;

            const DEM_IS_COMPRESSED: u32 = EDemoCommands::DemIsCompressed as u32;
            let is_compressed = c & DEM_IS_COMPRESSED == DEM_IS_COMPRESSED;

            let command = if is_compressed {
                c & !DEM_IS_COMPRESSED
            } else {
                c
            };

            (
                EDemoCommands::from_i32(command as i32).ok_or(Error::UnknownCmd(command))?,
                ot,
                is_compressed,
            )
        };

        let (tick, tick_ot) = {
            let (t, ot) = varint::read_uvarint32(&mut self.rdr)?;
            // before the first tick / pre-game initialization messages
            let tick = if t == u32::MAX { -1 } else { t as i32 };
            (tick, ot)
        };

        let (size, size_ot) = varint::read_uvarint32(&mut self.rdr)?;

        Ok(CmdHeader {
            command,
            is_compressed,
            tick,
            size,
            offset: command_ot + tick_ot + size_ot,
        })
    }

    // read_cmd reads bytes (cmd_header.size) from the reader into buf, if
    // compressed (cmd_header.is_compresed) it'll decompress the data.
    // NOTE: buf must be large enough to fit compressed and uncompressed data
    // simultaneously.
    pub fn read_cmd<'b>(
        &mut self,
        cmd_header: &CmdHeader,
        // TODO: change type of buf from &[u8] to Bytes to maybe avoid some
        // copying. see https://github.com/tokio-rs/prost/issues/571
        buf: &'b mut [u8],
    ) -> Result<&'b [u8]> {
        debug_assert!(
            self.demo_header.is_some(),
            "expected demo header to have been read"
        );

        let (left, right) = buf.split_at_mut(cmd_header.size as usize);
        self.rdr.read_exact(left)?;

        if cmd_header.is_compressed {
            let decompress_len = snap::raw::decompress_len(left)?;
            snap::raw::Decoder::new().decompress(left, right)?;
            // NOTE: we need to slice stuff up, because prost's decode can't
            // determine when to stop.
            Ok(&right[..decompress_len])
        } else {
            Ok(left)
        }
    }

    // void SeekTo( int position, bool bRead );
    //
    // copypasta from rust io::Seek (that is used under the hood):
    // > If the seek operation completed successfully, this method returns the
    // > new position from the start of the stream. That position can be used
    // > later with SeekFrom::Start.
    #[inline(always)]
    pub fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        self.rdr.seek(pos).map_err(Error::Io)
    }

    // unsigned int GetCurPos( bool bRead );
    #[inline(always)]
    pub fn stream_position(&mut self) -> Result<u64> {
        self.rdr.stream_position().map_err(Error::Io)
    }

    // int		GetSize();
    //
    // NOTE: Seek has stream_len method, but it's nigtly-only experimental
    // thing; at this point i would like to minimize use of non-stable features
    // (and bring then down to 0 eventually). what you see below is a copy-pasta
    // of rust's current implementation of it.
    //
    // quote from rust doc: > Note that length of a stream can change over time
    // (for example, when data is appended to a file). So calling this method
    // multiple times does not necessarily return the same length each time.
    pub fn stream_len(&mut self) -> Result<u64> {
        let old_pos = self.rdr.stream_position()?;
        let len = self.rdr.seek(SeekFrom::End(0))?;

        // Avoid seeking a third time when we were already at the end of the
        // stream. The branch is usually way cheaper than a seek operation.
        if old_pos != len {
            self.rdr.seek(SeekFrom::Start(old_pos))?;
        }

        Ok(len)
    }

    #[inline(always)]
    pub fn is_eof(&mut self) -> Result<bool> {
        Ok(self.stream_position()? == self.stream_len()?)
    }

    pub fn read_file_info<'buf>(
        &mut self,
        make_buf: impl FnOnce(&CmdHeader) -> &'buf mut [u8],
    ) -> Result<&CDemoFileInfo> {
        debug_assert!(
            self.demo_header.is_some(),
            "expected demo header to have been read"
        );

        let backup = self.stream_position()?;

        let file_info_offset =
            unsafe { self.demo_header.as_ref().unwrap_unchecked() }.fileinfo_offset;
        self.seek(SeekFrom::Start(file_info_offset as u64))?;

        let cmd_header = self.read_cmd_header()?;
        if cmd_header.command != EDemoCommands::DemFileInfo {
            return Err(Error::ExpectedCmd(EDemoCommands::DemFileInfo));
        }

        self.file_info = Some(CDemoFileInfo::decode(
            self.read_cmd(&cmd_header, make_buf(&cmd_header))?,
        )?);

        self.seek(SeekFrom::Start(backup))?;

        Ok(unsafe { self.file_info.as_ref().unwrap_unchecked() })
    }

    // virtual float GetTicksPerSecond() OVERRIDE;
    pub fn ticks_per_second(&self) -> f32 {
        debug_assert!(
            self.file_info.is_some(),
            "expected file info to have been read"
        );

        let file_info = unsafe { self.file_info.as_ref().unwrap_unchecked() };
        file_info.playback_ticks() as f32 / file_info.playback_time()
    }

    // virtual float GetTicksPerFrame() OVERRIDE;
    pub fn ticks_per_frame(&self) -> f32 {
        debug_assert!(
            self.file_info.is_some(),
            "expected file info to have been read"
        );

        let file_info = unsafe { self.file_info.as_ref().unwrap_unchecked() };
        file_info.playback_ticks() as f32 / file_info.playback_frames() as f32
    }

    // virtual int	GetTotalTicks( void ) OVERRIDE;
    pub fn total_ticks(&self) -> i32 {
        debug_assert!(
            self.file_info.is_some(),
            "expected file info to have been read"
        );

        let file_info = unsafe { self.file_info.as_ref().unwrap_unchecked() };
        file_info.playback_ticks()
    }

    pub fn skip(&mut self, cmd_header: &CmdHeader) -> Result<()> {
        self.seek(SeekFrom::Current(cmd_header.size as i64))
            .map(|_| ())
            .map_err(Error::from)
    }

    pub fn backup(&mut self, cmd_header: &CmdHeader) -> Result<()> {
        self.seek(SeekFrom::Current(-(cmd_header.offset as i64)))
            .map(|_| ())
            .map_err(Error::from)
    }
}
