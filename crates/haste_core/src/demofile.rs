use std::io::{self, Read, Seek, SeekFrom};

use dungers::varint;
use valveprotos::common::{CDemoFileInfo, EDemoCommands};
use valveprotos::prost::{self, Message};

// #define DEMO_RECORD_BUFFER_SIZE 2*1024*1024
//
// NOTE: read_cmd reads bytes (cmd_header.body_size) from the rdr into buf, if cmd is compressed
// (cmd_header.body_compressed) it'll decompress the data. buf must be large enough to fit
// compressed and uncompressed data simultaneously.
pub const DEMO_RECORD_BUFFER_SIZE: usize = 2 * 1024 * 1024;

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

/// can be used as a starting position for [`DemoFile`].
///
/// there might be situations when it is necessary to find something specific and then go back to
/// the very beginning (`demo_file.seek(SeekFrom::Start(DEMO_HEADER_SIZE))`).
pub const DEMO_HEADER_SIZE: usize = size_of::<DemoHeader>();

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

#[derive(thiserror::Error, Debug)]
pub enum ReadDemoHeaderError {
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error("invalid demo file stamp (got {got:?}; want id {DEMO_HEADER_ID:?})")]
    InvalidDemoFileStamp { got: [u8; DEMO_HEADER_ID_SIZE] },
}

#[derive(thiserror::Error, Debug)]
pub enum ReadCmdHeaderError {
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error(transparent)]
    ReadVarintError(#[from] varint::ReadVarintError),
    #[error("unknown cmd (raw {raw}; uncompressed {uncompressed})")]
    UnknownCmd { raw: u32, uncompressed: u32 },
}

#[derive(thiserror::Error, Debug)]
pub enum ReadCmdBodyError {
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error(transparent)]
    DecompressError(#[from] snap::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum ReadFileInfoError {
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error(transparent)]
    ReadCmdHeaderError(#[from] ReadCmdHeaderError),
    #[error(transparent)]
    ReadCmdBodyError(#[from] ReadCmdBodyError),
    #[error("unexpected cmd (got {got:?}; want {:?})", EDemoCommands::DemFileInfo)]
    UnexpectedCmd { got: EDemoCommands },
    #[error(transparent)]
    DecodeError(#[from] prost::DecodeError),
}

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
    /// # performance note
    ///
    /// for optimal performance make sure to provide a reader that implements buffering (for
    /// example [`std::io::BufReader`]).
    ///
    /// # usage note
    ///
    /// after creating a [`DemoFile`] instance, you must call [`DemoFile::read_demo_header`] before
    /// using any other methods. failure to do so will result in panics!
    pub fn from_reader(rdr: R) -> Self {
        Self {
            rdr,
            buf: vec![0u8; DEMO_RECORD_BUFFER_SIZE],
            demo_header: None,
            file_info: None,
        }
    }

    // stream ops
    // ----

    /// delegated from [`std::io::Seek`].
    #[inline]
    pub fn seek(&mut self, pos: SeekFrom) -> Result<u64, io::Error> {
        self.rdr.seek(pos)
    }

    /// delegated from [`std::io::Seek`].
    ///
    /// # note
    ///
    /// be aware that this method can be quite expensive. it might be best to make sure not to call
    /// it too frequently.
    #[inline]
    pub fn stream_position(&mut self) -> Result<u64, io::Error> {
        self.rdr.stream_position()
    }

    /// reimplementation of nightly [`std::io::Seek::stream_len`].
    pub fn stream_len(&mut self) -> Result<u64, io::Error> {
        let old_pos = self.rdr.stream_position()?;
        let len = self.rdr.seek(SeekFrom::End(0))?;

        // avoid seeking a third time when we were already at the end of the
        // stream. the branch is usually way cheaper than a seek operation.
        if old_pos != len {
            self.rdr.seek(SeekFrom::Start(old_pos))?;
        }

        Ok(len)
    }

    #[inline(always)]
    pub fn is_eof(&mut self) -> Result<bool, io::Error> {
        Ok(self.stream_position()? == self.stream_len()?)
    }

    // demo header
    // ----

    /// reads the demo header.
    ///
    /// this method should be called only once, immediately after creating a [`DemoFile`] instance.
    /// subsequent calls will result in panic.
    ///
    /// the read header is stored internally for future use and can be accessed using
    /// [`DemoFile::demo_header`] method.
    ///
    /// **Note:** do not use this when parsing HLTV fragments, instead use
    /// [`DemoFile::read_cmd_header_hltv`] directly from the start of the stream.
    pub fn read_demo_header(&mut self) -> Result<&DemoHeader, ReadDemoHeaderError> {
        assert!(
            self.demo_header.is_none(),
            "expected demo header not to have been read"
        );

        let mut demofilestamp = [0u8; DEMO_HEADER_ID_SIZE];
        self.rdr.read_exact(&mut demofilestamp)?;
        if demofilestamp != DEMO_HEADER_ID {
            return Err(ReadDemoHeaderError::InvalidDemoFileStamp { got: demofilestamp });
        }

        let mut buf = [0u8; size_of::<i32>()];

        self.rdr.read_exact(&mut buf)?;
        let fileinfo_offset = i32::from_le_bytes(buf);

        self.rdr.read_exact(&mut buf)?;
        let spawngroups_offset = i32::from_le_bytes(buf);

        self.demo_header = Some(DemoHeader {
            demofilestamp,
            fileinfo_offset,
            spawngroups_offset,
        });
        Ok(self.demo_header())
    }

    /// retrieves the demo header without performing any checks.
    ///
    /// this method should only be called after [`DemoFile::read_demo_header`] has been successfully
    /// executed. calling it before reading the header will result in panic.
    pub fn demo_header(&self) -> &DemoHeader {
        self.demo_header.as_ref().unwrap()
    }

    // cmd header
    // ----

    pub fn read_cmd_header(&mut self) -> Result<CmdHeader, ReadCmdHeaderError> {
        debug_assert!(
            self.demo_header.is_some(),
            "expected demo header to have been read"
        );

        let (cmd, cmd_n, body_compressed) = {
            let (cmd_raw, n) = varint::read_uvarint32(&mut self.rdr)?;

            const DEM_IS_COMPRESSED: u32 = EDemoCommands::DemIsCompressed as u32;
            let is_body_compressed = cmd_raw & DEM_IS_COMPRESSED == DEM_IS_COMPRESSED;

            let cmd = if is_body_compressed {
                cmd_raw & !DEM_IS_COMPRESSED
            } else {
                cmd_raw
            };

            (
                // TODO: do not perform useless work - do not convert command to enum, store it as
                // i32
                EDemoCommands::try_from(cmd as i32).map_err(|_| {
                    ReadCmdHeaderError::UnknownCmd {
                        raw: cmd_raw,
                        uncompressed: cmd,
                    }
                })?,
                n,
                is_body_compressed,
            )
        };

        let (tick, tick_n) = {
            let (tick, n) = varint::read_uvarint32(&mut self.rdr)?;
            // NOTE: tick is set to u32::MAX before before all pre-game initialization messages are
            // sent.
            // ticks everywhere are represented as i32, casting u32::MAX to i32 is okay because
            // bits in u32::MAX == bits in -1 i32.
            let tick = tick as i32;
            (tick, n)
        };

        let (body_size, body_size_n) = varint::read_uvarint32(&mut self.rdr)?;

        Ok(CmdHeader {
            cmd,
            body_compressed,
            tick,
            body_size,
            size: (cmd_n + tick_n + body_size_n) as u8,
        })
    }

    /// This is used to read HLTV demo fragments, *not* cmds from a normal Demo file.
    ///
    /// HLTV fragments can be parsed nearly the same as normal Demo files, however they have a
    /// slightly different way of separating commands, and they don't have a header.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut demo_file = DemoFile::from_reader(buf_reader);
    /// loop {
    ///     match demo_file.read_cmd_header_hltv() {
    ///         Ok(cmd_header) => {
    ///             eprintln!("{:?}", &cmd_header);
    ///             // read this in real code, of course
    ///             demo_file.skip_cmd_body(&cmd_header)?;
    ///         }
    ///         Err(err) => {
    ///             if demo_file.is_eof().unwrap_or_default() {
    ///                 println!("hit the end of the fragment(s)!");
    ///                 return Ok(());
    ///             }
    ///             return Err(err.into());
    ///         }
    ///     }
    /// }
    /// ```
    pub fn read_cmd_header_hltv(&mut self) -> Result<CmdHeader, ReadCmdHeaderError> {
        let (cmd, cmd_n, body_compressed) = {
            let (cmd_raw, n) = varint::read_uvarint32(&mut self.rdr)?;

            const DEM_IS_COMPRESSED: u32 = EDemoCommands::DemIsCompressed as u32;
            let is_body_compressed = cmd_raw & DEM_IS_COMPRESSED == DEM_IS_COMPRESSED;

            let cmd = if is_body_compressed {
                cmd_raw & !DEM_IS_COMPRESSED
            } else {
                cmd_raw
            };

            (
                // TODO: do not perform useless work - do not convert command to enum, store it as
                // i32
                EDemoCommands::try_from(cmd as i32).map_err(|_| {
                    ReadCmdHeaderError::UnknownCmd {
                        raw: cmd_raw,
                        uncompressed: cmd,
                    }
                })?,
                n,
                is_body_compressed,
            )
        };

        let mut buf = [0u8; size_of::<u32>()];

        let (tick, tick_n) = {
            self.rdr.read_exact(&mut buf)?;
            let tick = u32::from_le_bytes(buf) as i32;
            (tick, size_of::<u32>())
        };

        // https://github.com/saul/demofile-net/blob/7d3d59e478dbd2b000f4efa2dac70ed1bf2e2b7f/src/DemoFile/HttpBroadcastReader.cs#L150
        let (_unknown, unknown_n) = {
            self.rdr.read_exact(&mut buf[..1])?;
            (buf[0], 1)
        };

        let (body_size, body_size_n) = {
            self.rdr.read_exact(&mut buf)?;
            let body_size = u32::from_le_bytes(buf);
            (body_size, size_of::<u32>())
        };

        Ok(CmdHeader {
            cmd,
            body_compressed,
            tick,
            body_size,
            size: (cmd_n + tick_n + body_size_n + unknown_n) as u8,
        })
    }

    pub fn unread_cmd_header(&mut self, cmd_header: &CmdHeader) -> Result<(), io::Error> {
        self.seek(SeekFrom::Current(-(cmd_header.size as i64)))
            .map(|_| ())
    }

    // cmd body
    // ----

    pub fn read_cmd_body(&mut self, cmd_header: &CmdHeader) -> Result<&[u8], ReadCmdBodyError> {
        debug_assert!(
            self.demo_header.is_some(),
            "expected demo header to have been read"
        );

        let (left, right) = self.buf.split_at_mut(cmd_header.body_size as usize);
        self.rdr.read_exact(left)?;

        if cmd_header.body_compressed {
            let decompress_len = snap::raw::decompress_len(left)?;
            snap::raw::Decoder::new().decompress(left, right)?;
            // NOTE: we need to slice stuff up, because prost's decode can't
            // determine when to stop.
            Ok(&right[..decompress_len])
        } else {
            Ok(left)
        }
    }

    pub fn skip_cmd_body(&mut self, cmd_header: &CmdHeader) -> Result<(), io::Error> {
        self.seek(SeekFrom::Current(cmd_header.body_size as i64))
            .map(|_| ())
    }

    // file info
    // ----

    pub fn read_file_info(&mut self) -> Result<&CDemoFileInfo, ReadFileInfoError> {
        assert!(
            self.demo_header.is_some(),
            "expected demo header to have been read"
        );
        assert!(
            self.file_info.is_none(),
            "expected file info not to have been read"
        );

        let backup = self.stream_position()?;

        // NOTE: before any other method of DemoFile can be called consumer must call
        // read_demo_header method. it is safe to unwrap here because otherwise it is a failure of
        // consumer xd.
        let demo_header = self.demo_header();
        self.seek(SeekFrom::Start(demo_header.fileinfo_offset as u64))?;

        let cmd_header = self.read_cmd_header()?;
        if cmd_header.cmd != EDemoCommands::DemFileInfo {
            return Err(ReadFileInfoError::UnexpectedCmd {
                got: EDemoCommands::DemFileInfo,
            });
        }

        self.file_info = Some(CDemoFileInfo::decode(self.read_cmd_body(&cmd_header)?)?);

        self.seek(SeekFrom::Start(backup))?;

        Ok(self.unwrap_file_info())
    }

    // NOTE: file_info will call read_file_info if file info have not been read
    pub fn file_info(&mut self) -> Result<&CDemoFileInfo, ReadFileInfoError> {
        if self.file_info.is_none() {
            self.read_file_info()
        } else {
            Ok(self.unwrap_file_info())
        }
    }

    /// safe to use when you're sure that [`DemoFile::read_file_info`] was already called.
    pub fn unwrap_file_info(&self) -> &CDemoFileInfo {
        self.file_info.as_ref().unwrap()
    }

    // virtual float GetTicksPerSecond() OVERRIDE;
    pub fn ticks_per_second(&mut self) -> Result<f32, ReadFileInfoError> {
        let file_info = self.file_info()?;
        Ok(file_info.playback_ticks() as f32 / file_info.playback_time())
    }

    // virtual float GetTicksPerFrame() OVERRIDE;
    pub fn ticks_per_frame(&mut self) -> Result<f32, ReadFileInfoError> {
        let file_info = self.file_info()?;
        Ok(file_info.playback_ticks() as f32 / file_info.playback_frames() as f32)
    }

    // virtual int GetTotalTicks( void ) OVERRIDE;
    pub fn total_ticks(&mut self) -> Result<i32, ReadFileInfoError> {
        let file_info = self.file_info()?;
        Ok(file_info.playback_ticks())
    }
}
