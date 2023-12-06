use crate::{
    bitbuf::{self, BitReader},
    entityclasses::EntityClasses,
    fielddecoder, fieldpath,
    fieldvalue::FieldValue,
    flattenedserializers::{FlattenedSerializer, FlattenedSerializers},
    instancebaseline::InstanceBaseline,
    nohash::NoHashHasherBuilder,
};
use hashbrown::HashMap;
use std::rc::Rc;

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

#[derive(Debug, Clone)]
pub struct Entity {
    pub flattened_serializer: Rc<FlattenedSerializer>,
    pub field_values: HashMap<u64, FieldValue, NoHashHasherBuilder<u64>>,
}

impl Entity {
    fn parse(&mut self, br: &mut BitReader) -> Result<()> {
        // eprintln!("-- {}", self.flattened_serializer.serializer_name);

        fieldpath::FIELD_PATHS.with(|fps| {
            let mut fps = fps.borrow_mut();
            let fps = fieldpath::read_field_paths(br, &mut fps)?;
            for fp in fps {
                // eprint!("{:?} ", &fp.data[..=fp.position],);

                // NOTE: this loop performes much better then the unrolled
                // version of it, probably because a bunch of ifs cause a bunch
                // of branch misses and branch missles are disasterous.
                let mut field = self.flattened_serializer.get_child(fp.get(0));
                for i in 1..=fp.position {
                    field = field.get_child(fp.get(i));
                }

                // eprint!("{} {} ", field.var_name, field.var_type);

                // SAFETY: metadata for the field is generated in
                // flattenedserializers.rs; if metadata cannot be generated -
                // FlattenedSerializers::parse will fail thus we'll never get
                // here. it is safe to assume that field metadata cannot be
                // None.
                let field_metadata = unsafe { field.metadata.as_ref().unwrap_unchecked() };
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
                let field_key = unsafe { fp.hash_unchecked() };
                field_metadata.decoder.decode(br).map(|field_value| {
                    // eprintln!(" -> {:?}", &field_value);
                    self.field_values.insert(field_key, field_value);
                })?;
            }

            // dbg!(&self.field_values);
            // panic!();

            Ok(())
        })
    }
}

#[derive(Debug)]
pub struct Entities {
    // TODO: maybe use Vec<Option<.. or Vec<MaybeUninit<..
    //
    // but what if there will be more entities then the capacity&len that will
    // be pre-determined? some clues about max entry count can be found in
    // public/const.h (NUM_ENT_ENTRIES); clarity (and possibly manta) specify 1
    // << 14 as the max count of entries; butterfly uses number 20480.
    entities: HashMap<i32, Entity, NoHashHasherBuilder<i32>>,
}

impl Default for Entities {
    fn default() -> Self {
        Self {
            entities: HashMap::with_capacity_and_hasher(20480, NoHashHasherBuilder::default()),
        }
    }
}

impl Entities {
    pub fn handle_create(
        &mut self,
        entidx: i32,
        br: &mut BitReader,
        entity_classes: &EntityClasses,
        instance_baseline: &InstanceBaseline,
        flattened_serializers: &FlattenedSerializers,
    ) -> Result<&Entity> {
        let class_id = br.read_ubitlong(entity_classes.bits)? as i32;
        let _serial = br.read_ubitlong(17)?;
        let _unknown = br.read_uvarint32()?;

        let class_info = entity_classes.get_by_id(class_id).expect("class info");
        let flattened_serializer = flattened_serializers
            .get_by_serializer_name_hash(class_info.network_name_hash)
            .expect("flattened serializer");

        let mut entity = Entity {
            flattened_serializer: flattened_serializer.clone(),
            field_values: HashMap::with_capacity_and_hasher(
                flattened_serializer.fields.len(),
                NoHashHasherBuilder::default(),
            ),
        };

        // TODO: maybe it would make sense to cache baseline reads?
        let baseline_data = instance_baseline.get_data(class_id).expect("baseline data");
        let mut baseline_br = BitReader::new(baseline_data.as_ref());
        entity.parse(&mut baseline_br)?;

        entity.parse(br)?;

        self.entities.insert(entidx, entity);
        Ok(unsafe { self.entities.get(&entidx).unwrap_unchecked() })
    }

    pub fn handle_delete(&mut self, entidx: i32) -> Entity {
        // SAFETY: if it's being deleted menas that it was created, riiight?
        unsafe { self.entities.remove(&(entidx)).unwrap_unchecked() }
    }

    pub fn handle_update(&mut self, entidx: i32, br: &mut BitReader) -> Result<&Entity> {
        // SAFETY: if entity was ever created, and not deleted, it can be
        // updated!
        let entity = unsafe { self.entities.get_mut(&entidx).unwrap_unchecked() };
        entity.parse(br)?;
        Ok(entity)
    }

    // clear clears underlying storage, but this has no effect on the allocated
    // capacity.
    pub fn clear(&mut self) {
        self.entities.clear();
    }
}
