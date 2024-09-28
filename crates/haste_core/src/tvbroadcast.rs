use std::io::{self, Read, Seek, SeekFrom};

use valveprotos::common::EDemoCommands;

use crate::demofile::{CmdHeader, ReadCmdBodyError, ReadCmdHeaderError, DEMO_RECORD_BUFFER_SIZE};

// thanks to Bulbasaur (/ johnpyp) for bringing up tv broadcasts in discord, see
// https://discord.com/channels/1275127765879754874/1276578605836668969/1289323757403504734; and
// for beginning implementing support for them in https://github.com/blukai/haste/pull/2.

// useful links to dig into:
// - https://developer.valvesoftware.com/wiki/Counter-Strike:_Global_Offensive_Broadcast
// - https://github.com/saul/demofile-net/pull/93
// - https://github.com/FlowingSPDG/gotv-plus-go

// NOTE: it might make sense to put this behind feature-flag.

// TODO: start and keep reading broadcasts from http.

// TODO: intergrate this with Parser (parser.rs)?

// TODO: figure differences and similarities between tv broadcasts and demo files and see what can
// be generalized / combined with demo file.
//
// might want to treat demofile.rs thing not as really demo file, but as a "hub" for high level
// "components" that can be shared between different sources (file, tv broadcast, maybe realtime
// (/live) replay in the future as well (can be recorded via in-game console)).

#[derive(Debug)]
pub struct TvBroadcast<R: Read + Seek> {
    rdr: R,
    buf: Vec<u8>,
}

impl<R: Read + Seek> TvBroadcast<R> {
    /// creates a new [`TvBroadcast`] instance from the given reader.
    ///
    /// # performance note
    ///
    /// for optimal performance make sure to provide a reader that implements buffering (for
    /// example [`std::io::BufReader`]).
    pub fn from_reader(rdr: R) -> Self {
        Self {
            rdr,
            buf: vec![0u8; DEMO_RECORD_BUFFER_SIZE],
        }
    }

    // stream ops
    // ----
    //
    // TODO: stream ops are exactly the same as in DemoFile, see if they can be unified in a
    // reasonable way?

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

    // cmd header
    // ----
    //
    // NOTE: cmd headers are tv broadcasts are similar to demo file cmd headers, but encoding
    // is different.
    //
    // thanks to saul for figuring it out. see
    // https://github.com/saul/demofile-net/blob/7d3d59e478dbd2b000f4efa2dac70ed1bf2e2b7f/src/DemoFile/HttpBroadcastReader.cs#L150

    /// This is used to read HLTV demo fragments, *not* cmds from a normal Demo file.
    ///
    /// HLTV fragments can be parsed nearly the same as normal Demo files, however they have a
    /// slightly different way of separating commands, and they don't have a header.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut demo_file = TvBroadcast::from_reader(buf_reader);
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
    pub fn read_cmd_header(&mut self) -> Result<CmdHeader, ReadCmdHeaderError> {
        // TODO: bytereader (bitreader-like) + migrate read_exact and similar instalces across the
        // code base to it (valve have CUtlBuffer for reference to make api similar).
        let mut buf = [0u8; size_of::<u32>()];

        let (cmd, cmd_n, body_compressed) = {
            self.rdr.read_exact(&mut buf[..1])?;
            let cmd_raw = buf[0] as u32;

            const DEM_IS_COMPRESSED: u32 = EDemoCommands::DemIsCompressed as u32;
            let body_compressed = cmd_raw & DEM_IS_COMPRESSED == DEM_IS_COMPRESSED;

            let cmd = if body_compressed {
                cmd_raw & !DEM_IS_COMPRESSED
            } else {
                cmd_raw
            };

            (
                EDemoCommands::try_from(cmd as i32).map_err(|_| {
                    ReadCmdHeaderError::UnknownCmd {
                        raw: cmd_raw,
                        uncompressed: cmd,
                    }
                })?,
                1,
                body_compressed,
            )
        };

        let (tick, tick_n) = {
            self.rdr.read_exact(&mut buf)?;
            let tick = u32::from_le_bytes(buf) as i32;
            (tick, size_of::<u32>())
        };

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
    //
    // TODO: cmd body methods are exactly the same as in DemoFile, see if they can be unified in a
    // reasonable way?

    pub fn read_cmd_body(&mut self, cmd_header: &CmdHeader) -> Result<&[u8], ReadCmdBodyError> {
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
}
