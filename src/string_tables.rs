use crate::{protos, BitBuf};
use anyhow::Result;

// pub struct StringTable {
// 	string name;
// 	int32 num_entries;
// 	bool user_data_fixed_size;
// 	int32 user_data_size;
// 	int32 user_data_size_bits;
// 	int32 flags;
// 	bytes string_data;
// 	int32 uncompressed_size;
// 	bool data_compressed;
// }

pub struct StringTables {
    // by_name: HashMap<String, >
}

impl StringTables {
    pub fn new() -> Self {
        Self {}
    }

    pub fn insert(&mut self, proto: protos::CsvcMsgCreateStringTable) -> Result<()> {
        // NOTE: we are only interested in `instancebaseline` table
        if proto.name.as_ref().expect("some name") != "instancebaseline" {
            return Ok(());
        }

        let mut string_data = proto.string_data.expect("some string data");
        if proto.data_compressed.expect("some data compressed") {
            let size = snap::raw::decompress_len(&string_data)?;
            let mut dst = vec![0u8; size];
            snap::raw::Decoder::new().decompress(&string_data, &mut dst[..])?;
            string_data = dst;
        };

        parse_string_table(
            &string_data,
            proto.num_entries.expect("some num entries"),
            proto
                .user_data_fixed_size
                .expect("some user data fixed size"),
            proto.user_data_size_bits.expect("some user data size bits"),
            proto.flags.expect("some flags"),
        )?;

        Ok(())
    }
}

fn parse_string_table(
    string_data: &[u8],
    num_entries: i32,
    user_data_fixed_size: bool,
    user_data_size: i32,
    flags: i32,
) -> Result<()> {
    let mut bitbuf = BitBuf::new(string_data);

    // NOTE: some comments are stolen from manta.

    // - Each entry is a tuple consisting of {index, key, value}.
    // - Index can either be incremented from the previous position or
    // overwritten with a given entry.
    // - Key may be omitted
    // - Value may be omitted

    let mut index: i32 = -1;

    // printf debugging shows that keys are at most 4 bytes long.
    // we're reserving 1 extra byte to be able to consume null terminator /0.
    const MAX_KEY_SIZE: usize = 5;
    let mut key_buf = [0u8; MAX_KEY_SIZE];

    const MAX_KEY_HISTORY_SIZE: usize = 32;
    let mut keys = [([0u8; MAX_KEY_SIZE], 0usize); MAX_KEY_HISTORY_SIZE];

    let mut value_buf = [0u8; 0x4000];

    for i in 0..num_entries as usize {
        // Read a boolean to determine whether the operation is an increment or
        // has a fixed index position. A fixed index position of zero should be
        // the last data in the buffer, and indicates that all data has been
        // read.
        index = if bitbuf.read_bool()? {
            index + 1
        } else {
            bitbuf.read_varu32()? as i32 + 1
        };

        // Some values have keys, some don't.
        let key = if bitbuf.read_bool()? {
            // Some entries use reference a position in the key history for part
            // of the key. If referencing the history, read the position and
            // size from the buffer, then use those to build the string combined
            // with an extra string read (null terminated).
            let kl: ([u8; MAX_KEY_SIZE], usize) = if bitbuf.read_bool()? {
                // TODO: do butterfly's delta pos thing
                // TOOD: look how clarity does this
                let pos = bitbuf.read(5)? as usize;
                let size = bitbuf.read(5)? as usize;
                if pos >= keys.len() {
                    let key_len = bitbuf.read_str(&mut key_buf)?.len();
                    (key_buf, key_len)
                } else {
                    // this probably should beat butterfly and manta and clarity

                    let mut acc = [0u8; MAX_KEY_SIZE];
                    let mut j = 0;

                    let (pfx, pfx_len) = keys[pos];
                    let end = if size > pfx_len { pfx_len } else { size };
                    (0..end).into_iter().for_each(|k| {
                        acc[k] = pfx[k];
                        j += 1;
                    });

                    let sfx = bitbuf.read_str(&mut key_buf)?;
                    sfx.iter().for_each(|v| {
                        acc[j] = *v;
                        j += 1;
                    });

                    (acc, j)
                }
            } else {
                let key_len = bitbuf.read_str(&mut key_buf)?.len();
                (key_buf, key_len)
            };

            // TODO: if index of next key is > MAX_KEY_HISTORY_SIZE -> remove
            // index 0 and shift by 1 (or do something more efficient?
            // butterfly's delta trick?).
            keys[i] = kl;

            Some(std::str::from_utf8(&kl.0[..kl.1])?.to_owned())
        } else {
            None
        };

        dbg!(key);

        // Some entries have a value.
        if bitbuf.read_bool()? {
            if user_data_fixed_size {
                bitbuf.read_bits(&mut value_buf[..user_data_size as usize])?;
            } else {
                let is_compressed = if (flags & 0x1) != 0 {
                    bitbuf.read_bool()?
                } else {
                    false
                };

                let size = bitbuf.read(17)?;
                bitbuf.read_bytes(&mut value_buf[..size as usize])?;

                // TODO: uncompress data
            }
        }
    }

    Ok(())
}
