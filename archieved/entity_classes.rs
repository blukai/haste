use crate::protos;
use anyhow::Result;
use hashbrown::{hash_map::DefaultHashBuilder, HashMap};
use prost::Message;
use std::{
    alloc::Allocator,
    ops::{Deref, DerefMut},
};

type Container<A> = HashMap<i32, protos::c_demo_class_info::ClassT, DefaultHashBuilder, A>;

pub struct EntityClasses<A: Allocator + Clone> {
    container: Container<A>,
}

impl<A: Allocator + Clone> EntityClasses<A> {
    pub fn new_in(data: &[u8], alloc: A) -> Result<Self> {
        let class_info = protos::CDemoClassInfo::decode(data)?;
        let mut container = Container::with_capacity_in(class_info.classes.len(), alloc);
        for class in class_info.classes {
            container.insert(class.class_id.expect("some class id"), class);
        }
        Ok(Self { container })
    }

    // bits returns number of bits to read for entity classes.
    // stolen from butterfly's entity_classes.hpp.
    pub fn bits(&self) -> u32 {
        (self.container.len() as f32).log2().ceil() as u32
    }
}

impl<A: Allocator + Clone> Deref for EntityClasses<A> {
    type Target = Container<A>;

    fn deref(&self) -> &Self::Target {
        &self.container
    }
}

impl<A: Allocator + Clone> DerefMut for EntityClasses<A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.container
    }
}
