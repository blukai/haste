use valveprotos::common::CDemoClassInfo;

use crate::fxhash;

#[derive(Clone)]
pub struct ClassInfo {
    pub network_name_hash: u64,
}

pub struct EntityClasses {
    pub classes: usize,
    pub bits: usize,
    class_infos: Vec<ClassInfo>,
}

impl EntityClasses {
    pub fn parse(cmd: CDemoClassInfo) -> Self {
        let class_count = cmd.classes.len();

        // bits is the number of bits to read for entity classes. stolen from
        // butterfly's entity_classes.hpp.
        let bits = (class_count as f32).log2().ceil() as usize;

        let class_infos: Vec<ClassInfo> = cmd
            .classes
            .iter()
            .enumerate()
            .map(|(i, class)| {
                let class_id = class.class_id() as usize;
                debug_assert_eq!(class_id, i, "invliad class id");
                ClassInfo {
                    network_name_hash: fxhash::hash_bytes(class.network_name().as_bytes()),
                }
            })
            .collect();

        Self {
            classes: class_count,
            bits,
            class_infos,
        }
    }

    #[inline(always)]
    pub unsafe fn by_id_unckecked(&self, class_id: i32) -> &ClassInfo {
        self.class_infos.get_unchecked(class_id as usize)
    }
}
