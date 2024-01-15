use crate::{
    bitbuf::{self, BitReader},
    entityclasses::EntityClasses,
    fielddecoder, fieldpath,
    fieldvalue::FieldValue,
    flattenedserializers::{FlattenedSerializer, FlattenedSerializers},
    fxhash,
    instancebaseline::InstanceBaseline,
};
use hashbrown::{
    hash_map::{Entry, Values},
    HashMap,
};
use nohash::NoHashHasher;
use std::{hash::BuildHasherDefault, rc::Rc};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // crate
    #[error(transparent)]
    BitBuf(#[from] bitbuf::Error),
    #[error(transparent)]
    FieldPath(#[from] fieldpath::Error),
    #[error(transparent)]
    FieldDecoder(#[from] fielddecoder::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

// NOTE: PVS is potentially visible set,
// see more on https://developer.valvesoftware.com/wiki/PVS

// Flags for delta encoding header
// csgo src: engine/ents_shared.h
pub const FHDR_ZERO: usize = 0x0000;
pub const FHDR_LEAVEPVS: usize = 0x0001;
pub const FHDR_DELETE: usize = 0x0002;
pub const FHDR_ENTERPVS: usize = 0x0004;

// CL_ParseDeltaHeader in engine/client.cpp
#[inline(always)]
pub fn parse_delta_header(br: &mut BitReader) -> Result<usize> {
    let mut update_flags = FHDR_ZERO;
    // NOTE: read_bool is equivalent to ReadOneBit() == 1
    if !br.read_bool()? {
        if br.read_bool()? {
            update_flags |= FHDR_ENTERPVS;
        }
    } else {
        update_flags |= FHDR_LEAVEPVS;

        if br.read_bool()? {
            update_flags |= FHDR_DELETE;
        }
    }
    Ok(update_flags)
}

// Used to classify entity update types in DeltaPacketEntities.
// csgo src: engine/ents_shared.h
#[derive(Debug)]
pub enum UpdateType {
    EnterPVS = 0, // Entity came back into pvs, create new entity if one doesn't exist
    LeavePVS,     // Entity left pvs
    DeltaEnt,     // There is a delta for this entity.
}

// DetermineUpdateType in engine/client.cpp
#[inline(always)]
pub fn determine_update_type(update_flags: usize) -> UpdateType {
    if update_flags & FHDR_ENTERPVS != 0 {
        UpdateType::EnterPVS
    } else if update_flags & FHDR_LEAVEPVS != 0 {
        UpdateType::LeavePVS
    } else {
        UpdateType::DeltaEnt
    }
}

// TODO: do not publicly expose Entity's fields
#[derive(Debug, Clone)]
pub struct Entity {
    pub field_values: HashMap<u64, FieldValue, BuildHasherDefault<NoHashHasher<u64>>>,
    pub flattened_serializer: Rc<FlattenedSerializer>,
}

impl Entity {
    fn parse(&mut self, br: &mut BitReader) -> Result<()> {
        // eprintln!("-- {}", self.flattened_serializer.serializer_name);

        fieldpath::FIELD_PATHS.with(|fps| {
            let mut fps = unsafe { &mut *fps.get() };
            let fps = fieldpath::read_field_paths(br, &mut fps)?;
            for fp in fps {
                // eprint!("{:?} ", &fp.data[..=fp.position],);

                // NOTE: this loop performes much better then the unrolled
                // version of it, probably because a bunch of ifs cause a bunch
                // of branch misses and branch missles are disasterous.
                let mut field = unsafe {
                    self.flattened_serializer
                        .get_child_unchecked(fp.get_unchecked(0))
                };
                let mut field_key_hasher = fxhash::Hasher::new_with_seed(field.var_name_hash);
                for i in 1..=fp.position {
                    field = unsafe { field.get_child_unchecked(fp.get_unchecked(i)) };
                    field_key_hasher.write_u64(field.var_name_hash);
                }
                let field_key = field_key_hasher.finish();

                // eprint!("{} {} ", field.var_name, field.var_type);

                // NOTE: a shit ton of time was being spent here in a Try trait.
                // apparently Result is quite expensive xd. here's an artice
                // that i managed to find that talks more about the Try trait -
                // https://agourlay.github.io/rust-performance-retrospective-part3/
                //
                // tried couple of things here:
                // - unwrap_unchecked shaved off ~20 ms
                // - and_then (that was used by the author in the article above)
                //   did not really work here
                // - map shaved off ~40 ms without sacrafacing error checking, i
                //   have no idea why, but this is quite impressive at this
                //   point.
                field.metadata.decoder.decode(br).map(|field_value| {
                    // eprintln!(" -> {:?}", &field_value);
                    self.field_values.insert(field_key, field_value);
                })?;
            }

            // dbg!(&self.field_values);
            // panic!();

            Ok(())
        })
    }

    pub fn get_field_value(&self, field_key: u64) -> Option<&FieldValue> {
        self.field_values.get(&field_key)
    }
}

#[derive(Debug)]
pub struct EntityContainer {
    // NOTE: hashbrown hashmap with no hash performs better then Vec.
    entities: HashMap<i32, Entity, BuildHasherDefault<NoHashHasher<i32>>>,
    baseline_entities: HashMap<i32, Entity, BuildHasherDefault<NoHashHasher<i32>>>,
}

impl Default for EntityContainer {
    fn default() -> Self {
        Self {
            // NOTE: clarity (and possibly manta) specify 1 << 14 as the max
            // count of entries; butterfly uses number 20480.
            entities: HashMap::with_capacity_and_hasher(20480, BuildHasherDefault::default()),
            baseline_entities: HashMap::with_capacity_and_hasher(
                1024,
                BuildHasherDefault::default(),
            ),
        }
    }
}

impl EntityContainer {
    pub fn handle_create(
        &mut self,
        index: i32,
        br: &mut BitReader,
        entity_classes: &EntityClasses,
        instance_baseline: &InstanceBaseline,
        flattened_serializers: &FlattenedSerializers,
    ) -> Result<&Entity> {
        let class_id = br.read_ubitlong(entity_classes.bits)? as i32;
        let _serial = br.read_ubitlong(17)?;
        let _unknown = br.read_uvarint32()?;

        let class_info = unsafe { entity_classes.by_id_unckecked(class_id) };
        let flattened_serializer =
            unsafe { flattened_serializers.by_name_hash_unckecked(class_info.network_name_hash) };

        let mut entity = match self.baseline_entities.entry(class_id) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(e) => {
                let mut entity = Entity {
                    field_values: HashMap::with_capacity_and_hasher(
                        flattened_serializer.fields.len(),
                        BuildHasherDefault::default(),
                    ),
                    flattened_serializer,
                };
                let baseline_data = unsafe { instance_baseline.by_id_unchecked(class_id) };
                let mut baseline_br = BitReader::new(baseline_data.as_ref());
                entity.parse(&mut baseline_br)?;
                e.insert(entity).clone()
            }
        };

        entity.parse(br)?;

        self.entities.insert(index, entity);
        // SAFETY: the entity was just inserted ^, it's safe.
        Ok(unsafe { self.entities.get(&index).unwrap_unchecked() })
    }

    // SAFETY: if it's being deleted menas that it was created, riiight? but
    // there's a risk (that only should exist if replay is corrupted).
    #[inline]
    pub unsafe fn handle_delete_unchecked(&mut self, index: i32) -> Entity {
        unsafe { self.entities.remove(&(index)).unwrap_unchecked() }
    }

    // SAFETY: if entity was ever created, and not deleted, it can be updated!
    // but there's a risk (that only should exist if replay is corrupted).
    #[inline]
    pub unsafe fn handle_update_unchecked(
        &mut self,
        index: i32,
        br: &mut BitReader,
    ) -> Result<&Entity> {
        let entity = unsafe { self.entities.get_mut(&index).unwrap_unchecked() };
        entity.parse(br)?;
        Ok(entity)
    }

    // clear clears underlying storage, but this has no effect on the allocated
    // capacity.
    pub fn clear(&mut self) {
        self.entities.clear();
    }

    pub fn is_empty(&self) -> bool {
        // TODO: should entity_baselines be checked?
        self.entities.is_empty()
    }

    #[inline]
    pub fn values(&self) -> Values<'_, i32, Entity> {
        self.entities.values()
    }

    // TODO: introduce something like get_entity method
}

// ----

pub const fn make_field_key(path: &[&str]) -> u64 {
    assert!(path.len() > 0, "invalid path");
    let first = fxhash::hash_u8(path[0].as_bytes());
    let mut hasher = fxhash::Hasher::new_with_seed(first);
    let mut i = 1;
    while i < path.len() {
        let part = fxhash::hash_u8(path[i].as_bytes());
        hasher.write_u64(part);
        i += 1;
    }
    hasher.finish()
}
