use crate::{hashers::I32HashBuilder, stringtables::StringTable};
use hashbrown::HashMap;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // std
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
}

pub type Result<T> = std::result::Result<T, Error>;

pub const INSTANCE_BASELINE_TABLE_NAME: &str = "instancebaseline";

#[derive(Default)]
pub struct InstanceBaseline {
    map: HashMap<i32, Box<str>, I32HashBuilder>,
}

impl InstanceBaseline {
    pub fn update(&mut self, string_table: &StringTable) -> Result<()> {
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

            let class_id = string.parse::<i32>()?;
            self.map.insert(
                class_id,
                item.user_data.clone().expect("instance baseline data"),
            );
        }
        Ok(())
    }

    pub fn get_data(&self, class_id: i32) -> Option<&Box<str>> {
        self.map.get(&class_id)
    }
}
