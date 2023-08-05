use crate::{allocstring::AllocString, stringtables::StringTable};
use hashbrown::{hash_map::DefaultHashBuilder, HashMap};
use std::alloc::{Allocator, Global};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // std
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
}

pub type Result<T> = std::result::Result<T, Error>;

pub const INSTANCE_BASELINE_TABLE_NAME: &[u8] = b"instancebaseline";

pub struct InstanceBaseline<A: Allocator + Clone = Global> {
    map: HashMap<i32, AllocString<A>, DefaultHashBuilder, A>,
}

impl Default for InstanceBaseline<Global> {
    fn default() -> Self {
        Self::new_in(Global)
    }
}

impl<A: Allocator + Clone> InstanceBaseline<A> {
    pub fn new_in(alloc: A) -> Self {
        Self {
            map: HashMap::new_in(alloc),
        }
    }

    pub fn update(&mut self, string_table: &StringTable<A>) -> Result<()> {
        for (_entity_index, item) in string_table.iter() {
            let string = item
                .string
                .as_ref()
                .expect("instance baseline class id string");
            debug_assert!(
                string.len() <= 4,
                "unexpected len of instance baseline class id string: {}",
                string.len()
            );

            let class_id = string.as_str().parse::<i32>()?;
            self.map.insert(
                class_id,
                item.user_data.clone().expect("instance baseline data"),
            );
        }
        Ok(())
    }

    pub fn get_data(&self, class_id: i32) -> Option<&AllocString<A>> {
        self.map.get(&class_id)
    }
}
