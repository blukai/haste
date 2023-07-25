use crate::bitbuf::{self, BitReader};
use hashbrown::{
    hash_map::{DefaultHashBuilder, Iter},
    HashMap,
};
use std::alloc::{Allocator, Global};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // 3rd party crates
    #[error(transparent)]
    Snap(#[from] snap::Error),
    // crate
    #[error(transparent)]
    BitBuf(#[from] bitbuf::Error),
    // mod
    #[error("tried to create string table '{0}' twice")]
    DuplicateStringTable(String),
}

pub type Result<T> = std::result::Result<T, Error>;

const SUBSTRING_BITS: usize = 5;

#[derive(Default, Copy, Clone)]
struct StringHistoryEntry {
    string: [u8; 1 << SUBSTRING_BITS],
}

const MAX_USERDATA_BITS: usize = 17;
const MAX_USERDATA_SIZE: usize = 1 << MAX_USERDATA_BITS;

pub struct StringTableItem<A: Allocator + Clone> {
    pub string: Option<Vec<u8, A>>,
    pub user_data: Option<Vec<u8, A>>,
}

pub struct StringTable<A: Allocator + Clone> {
    pub name: Vec<u8, A>,
    user_data_fixed_size: bool,
    user_data_size: i32,
    user_data_size_bits: i32,
    flags: i32,
    using_varint_bitcounts: bool,
    items: HashMap<i32, StringTableItem<A>, DefaultHashBuilder, A>,
    alloc: A,
}

impl<A: Allocator + Clone> StringTable<A> {
    pub fn new_in(
        name: &str,
        user_data_fixed_size: bool,
        user_data_size: i32,
        user_data_size_bits: i32,
        flags: i32,
        using_varint_bitcounts: bool,
        alloc: A,
    ) -> Self {
        Self {
            name: name.as_bytes().to_vec_in(alloc.clone()),
            user_data_fixed_size,
            user_data_size,
            user_data_size_bits,
            flags,
            using_varint_bitcounts,
            items: HashMap::new_in(alloc.clone()),
            alloc,
        }
    }

    // void ParseUpdate( bf_read &buf, int entries );
    //
    // some pieces are ported from csgo, some are stolen from butterfly, some
    // comments are stolen from manta.
    pub fn parse_update(&mut self, br: &mut BitReader, num_entries: i32) -> Result<()> {
        let mut entry_index: i32 = -1;

        let mut history = [StringHistoryEntry::default(); 32];
        let mut history_delta_index: usize = 0;

        // Loop through entries in the data structure
        //
        // Each entry is a tuple consisting of {index, key, value}
        //
        // Index can either be incremented from the previous position or
        // overwritten with a given entry.
        //
        // Key may be omitted (will be represented here as "")
        //
        // Value may be omitted
        for _ in 0..num_entries as usize {
            // Read a boolean to determine whether the operation is an increment
            // or has a fixed index position. A fixed index position of zero
            // should be the last data in the buffer, and indicates that all
            // data has been read.
            entry_index = if br.read_bool()? {
                entry_index + 1
            } else {
                br.read_uvarint32()? as i32 + 1
            };

            let mut string_buf = [0u8; 1024];
            let has_string = br.read_bool()?;
            let string = if has_string {
                let mut size: usize = 0;

                // Some entries use reference a position in the key history for
                // part of the key. If referencing the history, read the
                // position and size from the buffer, then use those to build
                // the string combined with an extra string read (null
                // terminated). Alternatively, just read the string.
                if br.read_bool()? {
                    // NOTE: valve uses their CUtlVector which shifts elements
                    // to the left on delete. they maintain max len of 32. they
                    // don't allow history to grow beyond 32 elements, once it
                    // reaches len of 32 they remove item at index 0. i'm
                    // stealing following approach from butterfly, even thought
                    // rust's Vec has remove method which does exactly same
                    // thing, butterfly's way is more efficient, thanks!
                    let history_delta_zero = if history_delta_index > 32 {
                        history_delta_index & 31
                    } else {
                        0
                    };

                    let index = (history_delta_zero + br.read_ubitlong(5)? as usize) & 31;
                    let bytestocopy = br.read_ubitlong(SUBSTRING_BITS)? as usize;
                    size += bytestocopy;

                    string_buf[..bytestocopy]
                        .copy_from_slice(&history[index].string[..bytestocopy]);
                    size += br.read_string(&mut string_buf[bytestocopy..], false)?;
                } else {
                    size += br.read_string(&mut string_buf, false)?;
                }

                let mut she = StringHistoryEntry::default();
                let she_string_len = she.string.len();
                she.string.copy_from_slice(&string_buf[..she_string_len]);

                history[history_delta_index & 31] = she;
                history_delta_index += 1;

                Some(&string_buf[..size])
            } else {
                None
            };

            let mut user_data_buf = [0u8; MAX_USERDATA_SIZE];
            let has_user_data = br.read_bool()?;
            let user_data = if has_user_data {
                let mut size: usize;

                if self.user_data_fixed_size {
                    // Don't need to read length, it's fixed length and the length was networked down already.
                    size = self.user_data_size as usize;
                    br.read_bits(&mut user_data_buf, self.user_data_size_bits as usize)?;
                } else {
                    let is_compressed = if (self.flags & 0x1) != 0 {
                        br.read_bool()?
                    } else {
                        false
                    };

                    // NOTE: using_varint_bitcounts bool was introduced in the
                    // new frontiers update on smaypril twemmieth of 2023,
                    // https://github.com/SteamDatabase/GameTracking-Dota2/commit/8851e24f0e3ef0b618e3a60d276a3b0baf88568c#diff-79c9dd229c77c85f462d6d85e29a65f5daf6bf31f199554438d42bd643e89448R405
                    size = if self.using_varint_bitcounts {
                        br.read_ubitvar()?
                    } else {
                        br.read_ubitlong(MAX_USERDATA_BITS)?
                    } as usize;

                    br.read_bytes(&mut user_data_buf[..size])?;

                    if is_compressed {
                        let buf_copy = user_data_buf.clone();
                        snap::raw::Decoder::new()
                            .decompress(&buf_copy[..size], &mut user_data_buf)?;
                        size = snap::raw::decompress_len(&buf_copy)?;
                    }
                }

                Some(&user_data_buf[..size])
            } else {
                None
            };

            self.items.insert(
                entry_index,
                StringTableItem {
                    string: string.map(|s| s.to_vec_in(self.alloc.clone())),
                    user_data: user_data.map(|ud| ud.to_vec_in(self.alloc.clone())),
                },
            );
        }

        Ok(())
    }

    // NOTE: might need those for fast seeks
    // // HLTV change history & rollback
    // void EnableRollback();
    // void RestoreTick(int tick);

    pub fn iter(&self) -> Iter<'_, i32, StringTableItem<A>> {
        self.items.iter()
    }
}

pub struct StringTables<A: Allocator + Clone = Global> {
    tables: Vec<StringTable<A>, A>,
    alloc: A,
}

impl StringTables<Global> {
    pub fn new() -> Self {
        Self::new_in(Global)
    }
}

impl<A: Allocator + Clone> StringTables<A> {
    pub fn new_in(alloc: A) -> Self {
        Self {
            tables: Vec::new_in(alloc.clone()),
            alloc,
        }
    }

    // INetworkStringTable *CreateStringTable( const char *tableName, int maxentries, int userdatafixedsize = 0, int userdatanetworkbits = 0, int flags = NSF_NONE );
    pub fn create_string_table_mut(
        &mut self,
        name: &str,
        user_data_fixed_size: bool,
        user_data_size: i32,
        user_data_size_bits: i32,
        flags: i32,
        using_varint_bitcounts: bool,
    ) -> Result<&mut StringTable<A>> {
        let table = self.find_table(name);
        if table.is_some() {
            return Err(Error::DuplicateStringTable(name.to_string()));
        }

        let table = StringTable::new_in(
            name,
            user_data_fixed_size,
            user_data_size,
            user_data_size_bits,
            flags,
            using_varint_bitcounts,
            self.alloc.clone(),
        );

        let len = self.tables.len();
        self.tables.insert(len, table);
        Ok(&mut self.tables[len])
    }

    // INetworkStringTable *FindTable( const char *tableName ) const ;
    pub fn find_table(&self, name: &str) -> Option<&StringTable<A>> {
        for table in self.tables.iter() {
            if table.name.eq(name.as_bytes()) {
                return Some(table);
            }
        }
        None
    }

    // INetworkStringTable	*GetTable( TABLEID stringTable ) const;
    pub fn get_table(&self, id: usize) -> Option<&StringTable<A>> {
        self.tables.get(id)
    }

    pub fn get_table_mut(&mut self, id: usize) -> Option<&mut StringTable<A>> {
        self.tables.get_mut(id)
    }

    // NOTE: might need those for fast seeks
    // void EnableRollback( bool bState );
    // void RestoreTick( int tick );
}
