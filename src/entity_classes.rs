use crate::{
    error::{required, Result},
    protos,
};
use hashbrown::{hash_map::DefaultHashBuilder, HashMap};
use std::alloc::Allocator;

// see RecvTable_RecvClassInfos in engine/dt_recv_eng.cpp for refference

type Container<A> = HashMap<i32, protos::c_demo_class_info::ClassT, DefaultHashBuilder, A>;

pub struct EntityClasses<A: Allocator + Clone> {
    container: Container<A>,
    bits: u32,
}

impl<A: Allocator + Clone> EntityClasses<A> {
    pub fn new_in(proto: protos::CDemoClassInfo, alloc: A) -> Result<Self> {
        let n_classes = proto.classes.len();

        let mut container = Container::with_capacity_in(n_classes, alloc);
        for class in proto.classes {
            container.insert(class.class_id.ok_or(required!())?, class);
        }

        // bits is the number of bits to read for entity classes.
        // stolen from butterfly's entity_classes.hpp.
        let bits = (n_classes as f32).log2().ceil() as u32;

        Ok(Self { container, bits })
    }

    pub fn bits(&self) -> u32 {
        self.bits
    }

    #[inline(always)]
    pub fn get(&self, id: &i32) -> Option<&protos::c_demo_class_info::ClassT> {
        self.container.get(id)
    }
}
