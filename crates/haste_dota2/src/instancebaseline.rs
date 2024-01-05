use crate::stringtables::StringTable;

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
    data: Vec<Option<Vec<u8>>>,
}

impl InstanceBaseline {
    pub fn update(&mut self, string_table: &StringTable, classes: usize) -> Result<()> {
        if self.data.len() < classes {
            self.data.resize(classes, None);
        }

        for (_entity_index, item) in string_table.items() {
            let string = unsafe { std::str::from_utf8_unchecked(item.string.as_ref().unwrap_unchecked()) };
            let class_id = string.parse::<i32>()?;
            self.data[class_id as usize] = item.user_data.clone();
        }
        Ok(())
    }

    #[inline]
    pub fn get_data(&self, class_id: i32) -> Option<&Vec<u8>> {
        unsafe { self.data.get_unchecked(class_id as usize) }.as_ref()
    }

    // clear clears underlying storage, but this has no effect on the allocated
    // capacity.
    #[inline]
    pub fn clear(&mut self) {
        self.data.clear();
    }
}
