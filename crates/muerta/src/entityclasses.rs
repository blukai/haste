use crate::fnv1a;

#[derive(Clone)]
pub struct ClassInfo {
    pub network_name_hash: u64,
}

pub struct EntityClasses {
    pub classes: usize,
    pub bits: usize,
    class_infos: Vec<Option<ClassInfo>>,
}

impl EntityClasses {
    pub fn parse(proto: dota2protos::CDemoClassInfo) -> Self {
        let classes = proto.classes.len();

        // bits is the number of bits to read for entity classes. stolen from
        // butterfly's entity_classes.hpp.
        let bits = (classes as f32).log2().ceil() as usize;

        let mut class_infos = vec![None; classes];
        for class in proto.classes {
            let class_id = class.class_id.expect("class id");
            class_infos[class_id as usize] = Some(ClassInfo {
                network_name_hash: fnv1a::hash(class.network_name().as_bytes()),
            });
        }

        Self {
            classes,
            bits,
            class_infos,
        }
    }

    #[inline(always)]
    pub fn get_by_id(&self, id: i32) -> Option<&ClassInfo> {
        unsafe { self.class_infos.get_unchecked(id as usize).as_ref() }
    }
}
