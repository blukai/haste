use crate::{
    bitreader::{BitReader, BitReaderError},
    entityclasses::EntityClasses,
    fielddecoder,
    fieldpath::{self, FieldPath},
    fieldvalue::FieldValue,
    flattenedserializers::{
        FlattenedSerializer, FlattenedSerializerContainer, FlattenedSerializerField,
    },
    fxhash,
    instancebaseline::InstanceBaseline,
};
use hashbrown::{hash_map::Entry, HashMap};
use nohash::NoHashHasher;
use std::{hash::BuildHasherDefault, rc::Rc};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // crate
    #[error(transparent)]
    FieldDecoder(#[from] fielddecoder::Error),
    #[error(transparent)]
    BitReader(#[from] BitReaderError),
}

pub type Result<T> = std::result::Result<T, Error>;

// public/const.h (adjusted)

const MAX_EDICT_BITS: u32 = 14;
const MAX_EDICTS: u32 = 1 << MAX_EDICT_BITS;

const NUM_ENT_ENTRY_BITS: u32 = MAX_EDICT_BITS + 1;
const NUM_SERIAL_NUM_BITS: u32 = 32 - NUM_ENT_ENTRY_BITS;

const NUM_NETWORKED_EHANDLE_SERIAL_NUMBER_BITS: u32 = 10;
const NUM_NETWORKED_EHANDLE_BITS: u32 = MAX_EDICT_BITS + NUM_NETWORKED_EHANDLE_SERIAL_NUMBER_BITS;
const INVALID_NETWORKED_EHANDLE_VALUE: u32 = (1 << NUM_NETWORKED_EHANDLE_BITS) - 1;

// TODO: maybe introduce CHandle variant of FieldValue?

pub fn is_handle_valid(handle: u32) -> bool {
    handle != INVALID_NETWORKED_EHANDLE_VALUE
}

// game/client/recvproxy.cpp
// RecvProxy_IntToEHandle
// int iEntity = pData->m_Value.m_Int & ((1 << MAX_EDICT_BITS) - 1);
// int iSerialNum = pData->m_Value.m_Int >> MAX_EDICT_BITS;

pub fn handle_to_index(handle: u32) -> usize {
    (handle & ((1 << MAX_EDICT_BITS) - 1)) as usize
}

// TODO(blukai): investigate this (from public/basehandle.h):
// > The low NUM_SERIAL_BITS hold the index. If this value is less than MAX_EDICTS, then the entity is networkable.
// > The high NUM_SERIAL_NUM_BITS bits are the serial number.

// NOTE(blukai): idk, maybe to convert index and serial to handle do what CBaseHandle::Init (in
// public/basehandle.h) does:
// m_Index = iEntry | (iSerialNumber << NUM_SERIAL_NUM_SHIFT_BITS);

// TODO(blukai): get rid of a distinction between update type and update flags. read 2 bits and
// define 3 types of updates. that'll reduce branching further.

// NOTE: PVS is potentially visible set,
// see more on https://developer.valvesoftware.com/wiki/PVS
//
// Flags for delta encoding header
// csgo src: engine/ents_shared.h
pub const FHDR_ZERO: usize = 0x0000;
pub const FHDR_LEAVEPVS: usize = 0x0001;
pub const FHDR_DELETE: usize = 0x0002;
pub const FHDR_ENTERPVS: usize = 0x0004;

// CL_ParseDeltaHeader in engine/client.cpp
#[inline(always)]
pub(crate) fn parse_delta_header(br: &mut BitReader) -> Result<usize> {
    let mut update_flags = FHDR_ZERO;
    // NOTE: read_bool is equivalent to ReadOneBit() == 1
    if !br.read_bool() {
        if br.read_bool() {
            update_flags |= FHDR_ENTERPVS;
        }
    } else {
        update_flags |= FHDR_LEAVEPVS;

        if br.read_bool() {
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
pub(crate) fn determine_update_type(update_flags: usize) -> UpdateType {
    if update_flags & FHDR_ENTERPVS != 0 {
        UpdateType::EnterPVS
    } else if update_flags & FHDR_LEAVEPVS != 0 {
        UpdateType::LeavePVS
    } else {
        UpdateType::DeltaEnt
    }
}

#[derive(Debug, Clone)]
struct EntityField {
    #[cfg(feature = "preserve-metadata")]
    path: FieldPath,
    value: FieldValue,
}

// TODO: do not publicly expose Entity's fields
#[derive(Debug, Clone)]
pub struct Entity {
    index: i32,
    fields: HashMap<u64, EntityField, BuildHasherDefault<NoHashHasher<u64>>>,
    serializer: Rc<FlattenedSerializer>,
}

impl Entity {
    fn parse(&mut self, br: &mut BitReader, fps: &mut [FieldPath]) -> Result<()> {
        // eprintln!("-- {:?}", self.serializer.serializer_name);

        unsafe {
            let fp_count = fieldpath::read_field_paths(br, fps);
            for i in 0..fp_count {
                let fp = fps.get_unchecked(i);

                // eprint!("{:?} ", &fp.data[..=fp.last]);

                // NOTE: this loop performes much better then the unrolled
                // version of it, probably because a bunch of ifs cause a bunch
                // of branch misses and branch missles are disasterous.
                let mut field = self.serializer.get_child_unchecked(fp.get_unchecked(0));
                // NOTE: field.var_name.hash is a "seed" for field_key_hash.
                let mut field_key = field.var_name.hash;
                for i in 1..=fp.last() {
                    if field.is_dynamic_array() {
                        field = field.get_child_unchecked(0);
                        // NOTE: it's sort of weird to hash index, yup. but it simplifies things
                        // when "user" builds a key that has numbers / it makes it so that there's
                        // no need to check whether part of a key needs to be hashed or not - just
                        // hash all parts.
                        field_key = fxhash::add_u64_to_hash(
                            field_key,
                            fxhash::add_u64_to_hash(0, fp.get_unchecked(i) as u64),
                        );
                    } else {
                        field = field.get_child_unchecked(fp.get_unchecked(i));
                        field_key = fxhash::add_u64_to_hash(field_key, field.var_name.hash);
                    };
                }

                // eprint!("{:?} {:?} ", field.var_name, field.var_type);

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

                    match self.fields.entry(field_key) {
                        Entry::Occupied(mut oe) => {
                            oe.get_mut().value = field_value;
                        }
                        Entry::Vacant(ve) => {
                            ve.insert(EntityField {
                                #[cfg(feature = "preserve-metadata")]
                                path: fp.clone(),
                                value: field_value,
                            });
                        }
                    }
                })?;
            }

            // dbg!(&self.field_values);
            // panic!();
        }

        Ok(())
    }

    // ----

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&u64, &FieldValue)> {
        self.fields.iter().map(|(key, ef)| (key, &ef.value))
    }

    #[inline]
    pub fn get_value(&self, key: &u64) -> Option<&FieldValue> {
        self.fields.get(key).map(|ef| &ef.value)
    }

    #[cfg(feature = "preserve-metadata")]
    #[inline]
    pub fn get_path(&self, key: &u64) -> Option<&FieldPath> {
        self.fields.get(key).map(|ef| &ef.path)
    }

    // TODO: consider renaming this into just `serializer`.
    #[inline]
    pub fn serializer(&self) -> &FlattenedSerializer {
        self.serializer.as_ref()
    }

    #[inline]
    pub fn serializer_name_heq(&self, rhs: u64) -> bool {
        self.serializer.serializer_name.hash == rhs
    }

    #[inline]
    pub fn get_serializer_field(&self, path: &FieldPath) -> Option<&FlattenedSerializerField> {
        let first = path.get(0).and_then(|i| self.serializer.get_child(i));
        path.iter().skip(1).fold(first, |field, i| {
            field.and_then(|f| f.get_child(*i as usize))
        })
    }

    #[inline]
    pub fn index(&self) -> i32 {
        self.index
    }
}

#[derive(Debug)]
pub struct EntityContainer {
    // NOTE: hashbrown hashmap with no hash performs better then Vec.
    entities: HashMap<i32, Entity, BuildHasherDefault<NoHashHasher<i32>>>,
    baseline_entities: HashMap<i32, Entity, BuildHasherDefault<NoHashHasher<i32>>>,

    // NOTE: it might be tempting to introduce a "wrapper" struct, something like FieldPathReader
    // and turn read_field_path function into a method, but that's just suggar with no practical
    // benefit and extra indirection.
    // atm pointer to field_paths vec is being passed arround - that's 1 level. with theoretical
    // FieldPathsReader there would be 2 levels of indirection (at least as i imagine it right
    // now).
    field_paths: Vec<FieldPath>,
}

impl EntityContainer {
    pub(crate) fn new() -> Self {
        Self {
            entities: HashMap::with_capacity_and_hasher(
                // NOTE(blukai): in dota this value can be actually higher.
                MAX_EDICTS as usize,
                BuildHasherDefault::default(),
            ),
            baseline_entities: HashMap::with_capacity_and_hasher(
                1024,
                BuildHasherDefault::default(),
            ),
            // NOTE: 4096 is an arbitrary value that is large enough that that came out of printing
            // out count of fps collected per "run". (sort -nr can be handy)
            field_paths: vec![FieldPath::default(); 4096],
        }
    }

    pub(crate) fn handle_create(
        &mut self,
        index: i32,
        br: &mut BitReader,
        entity_classes: &EntityClasses,
        instance_baseline: &InstanceBaseline,
        serializers: &FlattenedSerializerContainer,
    ) -> Result<&Entity> {
        let class_id = br.read_ubit64(entity_classes.bits) as i32;
        let _serial = br.read_ubit64(NUM_SERIAL_NUM_BITS as usize);
        let _unknown = br.read_uvarint32();

        let class_info = unsafe { entity_classes.by_id_unckecked(class_id) };
        let serializer =
            unsafe { serializers.by_name_hash_unckecked(class_info.network_name_hash) };

        let mut entity = match self.baseline_entities.entry(class_id) {
            Entry::Occupied(oe) => {
                let mut entity = oe.get().clone();
                entity.index = index;
                entity
            }
            Entry::Vacant(ve) => {
                let mut entity = Entity {
                    index,
                    fields: HashMap::with_capacity_and_hasher(
                        serializer.fields.len(),
                        BuildHasherDefault::default(),
                    ),
                    serializer,
                };
                let baseline_data = unsafe { instance_baseline.by_id_unchecked(class_id) };

                let mut baseline_br = BitReader::new(baseline_data.as_ref());
                entity.parse(&mut baseline_br, &mut self.field_paths)?;
                baseline_br.is_overflowed()?;

                ve.insert(entity).clone()
            }
        };

        entity.parse(br, &mut self.field_paths)?;

        self.entities.insert(index, entity);
        // SAFETY: the entity was just inserted ^, it's safe.
        Ok(unsafe { self.entities.get(&index).unwrap_unchecked() })
    }

    // SAFETY: if it's being deleted menas that it was created, riiight? but
    // there's a risk (that only should exist if replay is corrupted).
    #[inline]
    pub(crate) unsafe fn handle_delete_unchecked(&mut self, index: i32) -> Entity {
        unsafe { self.entities.remove(&(index)).unwrap_unchecked() }
    }

    // SAFETY: if entity was ever created, and not deleted, it can be updated!
    // but there's a risk (that only should exist if replay is corrupted).
    #[inline]
    pub(crate) unsafe fn handle_update_unchecked(
        &mut self,
        index: i32,
        br: &mut BitReader,
    ) -> Result<&Entity> {
        let entity = unsafe { self.entities.get_mut(&index).unwrap_unchecked() };
        entity.parse(br, &mut self.field_paths)?;
        Ok(entity)
    }

    // ----

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&i32, &Entity)> {
        self.entities.iter()
    }

    #[inline]
    pub fn get(&self, index: &i32) -> Option<&Entity> {
        self.entities.get(index)
    }

    // clear clears underlying storage, but this has no effect on the allocated
    // capacity.
    #[inline]
    pub fn clear(&mut self) {
        self.entities.clear();
        self.baseline_entities.clear();
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }
}

// ----

pub const fn make_field_key(path: &[&str]) -> u64 {
    assert!(path.len() > 0, "invalid path");

    let seed = fxhash::hash_bytes(path[0].as_bytes());
    let mut hash = seed;

    let mut i = 1;
    while i < path.len() {
        let part = fxhash::hash_bytes(path[i].as_bytes());
        hash = fxhash::add_u64_to_hash(hash, part);
        i += 1;
    }

    hash
}
