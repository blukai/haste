use std::io::{Read, Seek, SeekFrom};

use dungers::varint;
use valveprotos::common::{CDemoFileInfo, EDemoCommands};
use valveprotos::prost::{self, Message};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // std
    #[error(transparent)]
    Io(#[from] std::io::Error),
    // external
    #[error(transparent)]
    Snap(#[from] snap::Error),
    #[error(transparent)]
    Prost(#[from] prost::DecodeError),
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

// #define DEMO_RECORD_BUFFER_SIZE 2*1024*1024
//
// NOTE: read_cmd reads bytes (cmd_header.size) from the rdr into buf, if cmd is
// compressed (cmd_header.is_compresed) it'll decompress the data. buf must be
// large enough to fit compressed and uncompressed data simultaneously.
pub const DEMO_BUFFER_SIZE: usize = 2 * 1024 * 1024;

// NOTE: DEMO_HEADER_SIZE can be used as a starting position for DemoFile. there
// might be situations when it is necessary to find something specific and then
// go back to the very beginning /
// demo_file.seek(SeekFrom::Start(DEMO_HEADER_SIZE))
pub const DEMO_HEADER_SIZE: usize = std::mem::size_of::<DemoHeader>();

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

#[derive(Debug, Clone)]
pub struct CmdHeader {
    // TODO: do not perform useless work - do not convert command to enum, store
    // it as i32
    pub command: EDemoCommands,
    pub is_compressed: bool,
    pub tick: i32,
    pub size: u32,
    // bytes_read is how many bytes were read. bytes_read can be used to do a
    // backup (/unread cmd header) - calculate bytes_read is cheaper then
    // calling stream_position method before reading cmd header.
    pub bytes_read: usize,
}

// NOTE: you should provide a reader that implements buffering (eg BufReader)
// because it'll be much more efficient.

#[derive(Debug)]
pub struct DemoFile<R: Read + Seek> {
    rdr: R,
    buf: Vec<u8>,
    demo_header: Option<DemoHeader>,
    file_info: Option<CDemoFileInfo>,
}

impl<R: Read + Seek> DemoFile<R> {
    /// creates a new [`DemoFile`] instance from the given reader.
    ///
    /// # note
    ///
    /// after creating a [`DemoFile`] instance, you must call [`Self::read_demo_header`] before
    /// using any other methods. failure to do so will result in panics!
    pub fn from_reader(rdr: R) -> Self {
        Self {
            rdr,
            buf: vec![0u8; DEMO_BUFFER_SIZE],
            demo_header: None,
            file_info: None,
        }
    }

    // ----

    // void SeekTo( int position, bool bRead );
    //
    /// delegated from [`std::io::Seek`].
    #[inline]
    pub fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        self.rdr.seek(pos).map_err(Error::Io)
    }

    // unsigned int GetCurPos( bool bRead );
    //
    /// delegated from [`std::io::Seek`].
    ///
    /// # note
    ///
    /// be aware that this method can be quite expensive. it might be best to make sure not to call
    /// it too frequently.
    #[inline]
    pub fn stream_position(&mut self) -> Result<u64> {
        self.rdr.stream_position().map_err(Error::Io)
    }

    // int GetSize();
    //
    /// reimplementation of nightly [`std::io::Seek::stream_len`].
    pub fn stream_len(&mut self) -> Result<u64> {
        let old_pos = self.rdr.stream_position()?;
        let len = self.rdr.seek(SeekFrom::End(0))?;

        // avoid seeking a third time when we were already at the end of the
        // stream. the branch is usually way cheaper than a seek operation.
        if old_pos != len {
            self.rdr.seek(SeekFrom::Start(old_pos))?;
        }

        Ok(len)
    }

    /// when continuously reading cmds in a loop this method can help to determine when to stop.
    #[inline(always)]
    pub fn is_eof(&mut self) -> Result<bool> {
        Ok(self.stream_position()? == self.stream_len()?)
    }

    // ----

    // demoheader_t* ReadDemoHeader( CDemoPlaybackParameters_t const *pPlaybackParameters );
    pub fn read_demo_header(&mut self) -> Result<&DemoHeader> {
        debug_assert!(
            self.demo_header.is_none(),
            "expected demo header not to have been read"
        );

        let mut demofilestamp = [0u8; DEMO_HEADER_ID_SIZE];
        self.rdr.read_exact(&mut demofilestamp)?;
        if demofilestamp != DEMO_HEADER_ID {
            return Err(Error::UnexpectedHeaderId {
                want: DEMO_HEADER_ID,
                got: demofilestamp,
            });
        }

        let mut buf = [0u8; 4];

        self.rdr.read_exact(&mut buf)?;
        let fileinfo_offset = i32::from_le_bytes(buf);

        self.rdr.read_exact(&mut buf)?;
        let spawngroups_offset = i32::from_le_bytes(buf);

        self.demo_header = Some(DemoHeader {
            demofilestamp,
            fileinfo_offset,
            spawngroups_offset,
        });

        Ok(self.unwrap_demo_header())
    }

    // NOTE: demo_header will call read_demo_header if demo header have not been
    // read
    pub fn demo_header(&mut self) -> Result<&DemoHeader> {
        if self.demo_header.is_none() {
            self.read_demo_header()
        } else {
            Ok(self.unwrap_demo_header())
        }
    }

    /// safe to use when you're sure that [`Self::read_demo_header`] was already called.
    pub fn unwrap_demo_header(&self) -> &DemoHeader {
        self.demo_header.as_ref().unwrap()
    }

    // ----

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
                EDemoCommands::try_from(command as i32).map_err(|_| Error::UnknownCmd(command))?,
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
            bytes_read: command_ot + tick_ot + size_ot,
        })
    }

    pub fn unread_cmd_header(&mut self, cmd_header: &CmdHeader) -> Result<()> {
        self.seek(SeekFrom::Current(-(cmd_header.bytes_read as i64)))
            .map(|_| ())
            .map_err(Error::from)
    }

    pub fn read_cmd(&mut self, cmd_header: &CmdHeader) -> Result<&[u8]> {
        debug_assert!(
            self.demo_header.is_some(),
            "expected demo header to have been read"
        );

        let (left, right) = self.buf.split_at_mut(cmd_header.size as usize);
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

    pub fn skip_cmd(&mut self, cmd_header: &CmdHeader) -> Result<()> {
        self.seek(SeekFrom::Current(cmd_header.size as i64))
            .map(|_| ())
            .map_err(Error::from)
    }

    // ----

    pub fn read_file_info(&mut self) -> Result<&CDemoFileInfo> {
        debug_assert!(
            self.demo_header.is_some(),
            "expected demo header to have been read"
        );
        debug_assert!(
            self.file_info.is_none(),
            "expected file info not to have been read"
        );

        let backup = self.stream_position()?;

        // NOTE: before any other method of DemoFile can be called consumer must call
        // read_demo_header method. it is safe to unwrap here because otherwise it is a failure of
        // consumer xd.
        let demo_header = self.unwrap_demo_header();
        self.seek(SeekFrom::Start(demo_header.fileinfo_offset as u64))?;

        let cmd_header = self.read_cmd_header()?;
        if cmd_header.command != EDemoCommands::DemFileInfo {
            return Err(Error::ExpectedCmd(EDemoCommands::DemFileInfo));
        }

        self.file_info = Some(CDemoFileInfo::decode(self.read_cmd(&cmd_header)?)?);

        self.seek(SeekFrom::Start(backup))?;

        Ok(self.unwrap_file_info())
    }

    // NOTE: file_info will call read_file_info if file info have not been read
    pub fn file_info(&mut self) -> Result<&CDemoFileInfo> {
        if self.file_info.is_none() {
            self.read_file_info()
        } else {
            Ok(self.unwrap_file_info())
        }
    }

    /// safe to use when you're sure that [`Self::read_file_info`] was already called.
    pub fn unwrap_file_info(&self) -> &CDemoFileInfo {
        self.file_info.as_ref().unwrap()
    }

    // virtual float GetTicksPerSecond() OVERRIDE;
    pub fn ticks_per_second(&mut self) -> Result<f32> {
        let file_info = self.file_info()?;
        Ok(file_info.playback_ticks() as f32 / file_info.playback_time())
    }

    // virtual float GetTicksPerFrame() OVERRIDE;
    pub fn ticks_per_frame(&mut self) -> Result<f32> {
        let file_info = self.file_info()?;
        Ok(file_info.playback_ticks() as f32 / file_info.playback_frames() as f32)
    }

    // virtual int	GetTotalTicks( void ) OVERRIDE;
    pub fn total_ticks(&mut self) -> Result<i32> {
        let file_info = self.file_info()?;
        Ok(file_info.playback_ticks())
    }
}
