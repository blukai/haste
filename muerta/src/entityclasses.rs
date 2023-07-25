use crate::{fnv1a, protos};
use hashbrown::{hash_map::DefaultHashBuilder, HashMap};
use std::alloc::{Allocator, Global};

pub struct ClassInfo {
    pub network_name_hash: u64,
}

type ClassInfoMap<A> = HashMap<i32, ClassInfo, DefaultHashBuilder, A>;

pub struct EntityClasses<A: Allocator + Clone = Global> {
    class_infos: Option<ClassInfoMap<A>>,
    bits: Option<i32>,
    alloc: A,
}

impl EntityClasses<Global> {
    pub fn new() -> Self {
        Self::new_in(Global)
    }
}

impl<A: Allocator + Clone> EntityClasses<A> {
    pub fn new_in(alloc: A) -> Self {
        Self {
            class_infos: None,
            bits: None,
            alloc,
        }
    }

    pub fn parse(&mut self, proto: protos::CDemoClassInfo) {
        debug_assert!(
            self.class_infos.is_none(),
            "class info map is expected to not be created yet"
        );

        let n_classes = proto.classes.len();
        let mut class_infos = ClassInfoMap::with_capacity_in(n_classes, self.alloc.clone());
        for class in proto.classes {
            let class_info = ClassInfo {
                network_name_hash: fnv1a::hash(class.network_name().as_bytes()),
            };
            class_infos.insert(class.class_id.expect("class id"), class_info);
        }
        self.bits = Some((class_infos.len() as f32).log2().ceil() as i32);
        self.class_infos = Some(class_infos);
    }

    #[inline(always)]
    fn class_infos(&self) -> &ClassInfoMap<A> {
        self.class_infos.as_ref().expect("class infos to be parsed")
    }

    // bits is the number of bits to read for entity classes.
    // stolen from butterfly's entity_classes.hpp.
    #[inline(always)]
    pub fn bits(&self) -> i32 {
        self.bits.expect("bits bit be set")
    }

    #[inline(always)]
    pub fn get_by_id(&self, id: &i32) -> Option<&ClassInfo> {
        self.class_infos().get(id)
    }
}
