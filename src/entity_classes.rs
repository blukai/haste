use crate::protos;
use anyhow::Result;
use prost::Message;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

type HM = HashMap<i32, protos::c_demo_class_info::ClassT>;

pub struct EntityClasses(HM);

impl EntityClasses {
    pub fn new(data: &[u8]) -> Result<Self> {
        let class_info = protos::CDemoClassInfo::decode(data)?;
        let mut hm = HM::with_capacity(class_info.classes.len());
        for class in class_info.classes {
            hm.insert(class.class_id.expect("some class id"), class);
        }
        Ok(Self(hm))
    }

    // bits returns number of bits to read for entity classes.
    // stolen from butterfly's entity_classes.hpp.
    pub fn bits(&self) -> u32 {
        (self.0.len() as f32).log2().ceil() as u32
    }
}

impl Deref for EntityClasses {
    type Target = HM;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for EntityClasses {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
