use crate::{
    bitreader::BitReader,
    error::{required, Result},
    protos,
};
use compact_str::CompactString;
use hashbrown::{hash_map::DefaultHashBuilder, HashMap};
use std::alloc::Allocator;

pub struct StringTableEntry<A: Allocator> {
    pub index: i32,
    pub key: Option<CompactString>,
    pub value: Option<Vec<u8, A>>,
}

// key is index
pub type StringTable<A> = HashMap<i32, StringTableEntry<A>, DefaultHashBuilder, A>;

type Container<A> = HashMap<CompactString, StringTable<A>, DefaultHashBuilder, A>;

pub struct StringTables<A: Allocator + Clone> {
    container: Container<A>,
    alloc: A,
}

impl<A: Allocator + Clone> StringTables<A> {
    pub fn new_in(alloc: A) -> Self {
        Self {
            container: Container::new_in(alloc.clone()),
            alloc,
        }
    }

    pub fn create(&mut self, proto: protos::CsvcMsgCreateStringTable) -> Result<&StringTable<A>> {
        let mut string_data = proto.string_data.ok_or(required!())?;
        if proto.data_compressed.ok_or(required!())? {
            let size = snap::raw::decompress_len(&string_data)?;
            let mut dst = vec![0u8; size];
            snap::raw::Decoder::new().decompress(&string_data, &mut dst[..])?;
            string_data = dst;
        };

        let string_table = self.handle_string_table(
            CompactString::from(&proto.name.ok_or(required!())?),
            &string_data,
            proto.num_entries.ok_or(required!())?,
            proto.user_data_fixed_size.ok_or(required!())?,
            proto.user_data_size_bits.expect("some user data size bits"),
            proto.flags.ok_or(required!())?,
            proto.using_varint_bitcounts.ok_or(required!())?,
        )?;

        Ok(string_table)
    }

    // TODO: make this merhod nicer (/look into it)
    fn handle_string_table(
        &mut self,
        name: CompactString,
        string_data: &[u8],
        num_entries: i32,
        user_data_fixed_size: bool,
        user_data_size: i32,
        flags: i32,
        using_varint_bitcounts: bool,
    ) -> Result<&StringTable<A>> {
        if !self.container.contains_key(&name) {
            self.container
                .insert(name.clone(), StringTable::new_in(self.alloc.clone()));
        }
        let string_table = self.container.get_mut(&name).unwrap();

        let mut br = BitReader::new(string_data);

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

        // NOTE: keyh stands for key history
        const KEYH_SIZE: usize = 32;
        const KEYH_MASK: usize = KEYH_SIZE - 1;
        let mut keyh = [[0u8; MAX_KEY_SIZE]; KEYH_SIZE];
        // NOTE: delta trick is stolen from butterfly
        let mut keyh_delta_pos = 0;

        let mut value_buf = [0u8; 0x4000];
        let mut value_snappy_buf = [0u8; 0x4000];

        for _ in 0..num_entries as usize {
            // Read a boolean to determine whether the operation is an increment or
            // has a fixed index position. A fixed index position of zero should be
            // the last data in the buffer, and indicates that all data has been
            // read.
            index = if br.read_bool()? {
                index + 1
            } else {
                br.read_varu32()? as i32 + 1
            };

            // Some values have keys, some don't.
            let key = if br.read_bool()? {
                // Some entries use reference a position in the key history for part
                // of the key. If referencing the history, read the position and
                // size from the buffer, then use those to build the string combined
                // with an extra string read (null terminated).
                let kl: ([u8; MAX_KEY_SIZE], usize) = if br.read_bool()? {
                    let keyh_delta_zero = if keyh_delta_pos > KEYH_SIZE {
                        keyh_delta_pos & KEYH_MASK
                    } else {
                        0
                    };
                    let keyh_pos = (keyh_delta_zero + br.read(5)? as usize) & KEYH_MASK;
                    let keyh_len = br.read(5)? as usize;

                    let mut key_buf = keyh[keyh_pos];
                    let len = br.read_string(&mut key_buf[keyh_len..])?.len();
                    (key_buf, keyh_len + len)
                } else {
                    let len = br.read_string(&mut key_buf)?.len();
                    (key_buf, len)
                };

                keyh[keyh_delta_pos & KEYH_MASK] = kl.0;
                keyh_delta_pos += 1;

                Some(CompactString::from_utf8(&kl.0[..kl.1])?)
            } else {
                None
            };

            // Some entries have a value.
            let value = if br.read_bool()? {
                let tmp = if user_data_fixed_size {
                    let value = &mut value_buf[..user_data_size as usize];
                    br.read_bits(value)?;
                    value
                } else {
                    let is_compressed = if (flags & 0x1) != 0 {
                        br.read_bool()?
                    } else {
                        false
                    };
                    // NOTE: using_varint_bitcounts bool was introduced in the
                    // new frontiers update on smaypril twemmieth,
                    // https://github.com/SteamDatabase/GameTracking-Dota2/commit/8851e24f0e3ef0b618e3a60d276a3b0baf88568c#diff-79c9dd229c77c85f462d6d85e29a65f5daf6bf31f199554438d42bd643e89448R405
                    let size = if using_varint_bitcounts {
                        br.read_ubitvar()?
                    } else {
                        br.read(17)?
                    };

                    let value = &mut value_buf[..size as usize];
                    br.read_bytes(value)?;

                    if is_compressed {
                        let dec_size = snap::raw::decompress_len(value)?;
                        let dec_value = &mut value_snappy_buf[..dec_size];
                        snap::raw::Decoder::new().decompress(value, dec_value)?;
                        dec_value
                    } else {
                        value
                    }
                };
                let result = tmp.to_vec_in(self.alloc.clone());
                Some(result)
            } else {
                None
            };

            string_table.insert(index, StringTableEntry { index, key, value });
        }

        Ok(string_table)
    }

    #[inline(always)]
    pub fn get(&self, key: &str) -> Option<&StringTable<A>> {
        self.container.get(key)
    }
}
