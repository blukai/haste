use crate::{
    bitbuf::{self, BitReader},
    entityclasses::EntityClasses,
    fielddecoder,
    fieldpath::{self},
    fieldvalue::FieldValue,
    flattenedserializers::{FlattenedSerializer, FlattenedSerializers},
    instancebaseline::InstanceBaseline,
    protos,
};
use hashbrown::{hash_map::DefaultHashBuilder, HashMap};
use std::alloc::{Allocator, Global};

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
const FHDR_ZERO: usize = 0x0000;
const FHDR_LEAVEPVS: usize = 0x0001;
const FHDR_DELETE: usize = 0x0002;
const FHDR_ENTERPVS: usize = 0x0004;

// CL_ParseDeltaHeader in engine/client.cpp
#[inline(always)]
fn parse_delta_header(br: &mut BitReader) -> Result<usize> {
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
enum UpdateType {
    EnterPVS = 0, // Entity came back into pvs, create new entity if one doesn't exist
    LeavePVS,     // Entity left pvs
    DeltaEnt,     // There is a delta for this entity.
}

// DetermineUpdateType in engine/client.cpp
#[inline(always)]
fn determine_update_type(update_flags: usize) -> UpdateType {
    if update_flags & FHDR_ENTERPVS != 0 {
        UpdateType::EnterPVS
    } else if update_flags & FHDR_LEAVEPVS != 0 {
        UpdateType::LeavePVS
    } else {
        UpdateType::DeltaEnt
    }
}

#[derive(Clone)]
pub struct Entity<A: Allocator + Clone> {
    flattened_serializer: FlattenedSerializer<A>,
    field_values: HashMap<u64, FieldValue<A>, DefaultHashBuilder, A>,
    alloc: A,
}

impl<A: Allocator + Clone> Entity<A> {
    fn parse(&mut self, br: &mut BitReader) -> Result<()> {
        let fps = fieldpath::read_field_paths_in(br, self.alloc.clone())?;
        for fp in fps {
            let field = self.flattened_serializer.get_field_for_field_path(fp, 0);

            let field_value = field
                .decoder
                .map(|decoder| decoder(field, br, self.alloc.clone()))
                .transpose()?
                .unwrap_or_else(|| {
                    panic!(
                        "field value (var_name: {}; var_type: {})",
                        unsafe { std::str::from_utf8_unchecked(&field.var_name) },
                        unsafe { std::str::from_utf8_unchecked(&field.var_type) }
                    )
                });

            self.field_values.insert(field.var_name_hash, field_value);
        }

        Ok(())
    }
}

pub struct Entities<A: Allocator + Clone = Global> {
    entities: HashMap<usize, Entity<A>, DefaultHashBuilder, A>,
    alloc: A,
}

impl Default for Entities<Global> {
    fn default() -> Self {
        Self::new_in(Global)
    }
}

impl<A: Allocator + Clone> Entities<A> {
    pub fn new_in(alloc: A) -> Self {
        Self {
            entities: HashMap::new_in(alloc.clone()),
            alloc,
        }
    }

    pub fn read_packet_entities(
        &mut self,
        svcmsg: protos::CsvcMsgPacketEntities,
        entity_classes: &EntityClasses<A>,
        instance_baseline: &InstanceBaseline<A>,
        flattened_serializers: &FlattenedSerializers<A>,
    ) -> Result<()> {
        let entity_data = svcmsg.entity_data.expect("entity data");
        let mut br = BitReader::new(&entity_data);

        let mut entidx: i32 = -1;
        for _ in (0..svcmsg.updated_entries.expect("updated entries")).rev() {
            entidx += br.read_ubitvar()? as i32 + 1;

            let update_flags = parse_delta_header(&mut br)?;
            let update_type = determine_update_type(update_flags);

            match update_type {
                UpdateType::EnterPVS => {
                    self.handle_create(
                        entidx as usize,
                        &mut br,
                        entity_classes,
                        instance_baseline,
                        flattened_serializers,
                    )?;
                }
                UpdateType::LeavePVS => {
                    if (update_flags & FHDR_DELETE) != 0 {
                        self.entities.remove(&(entidx as usize));
                    }
                }
                UpdateType::DeltaEnt => {
                    self.handle_update(entidx as usize, &mut br)?;
                }
            }
        }

        Ok(())
    }

    fn handle_create(
        &mut self,
        entity_index: usize,
        br: &mut BitReader,
        entity_classes: &EntityClasses<A>,
        instance_baseline: &InstanceBaseline<A>,
        flattened_serializers: &FlattenedSerializers<A>,
    ) -> Result<()> {
        let class_id = br.read_ubitlong(entity_classes.bits() as usize)? as i32;
        let _serial = br.read_ubitlong(17)?;
        let _unknown = br.read_uvarint32()?;

        let class_info = entity_classes.get_by_id(&class_id).expect("class info");
        let flattened_serializer = flattened_serializers
            .get_by_serializer_name_hash(class_info.network_name_hash)
            .expect("flattened serializer")
            .clone();

        let field_values =
            HashMap::with_capacity_in(flattened_serializer.fields.len(), self.alloc.clone());

        let mut entity = Entity {
            flattened_serializer,
            field_values,
            alloc: self.alloc.clone(),
        };

        let mut baseline_br =
            BitReader::new(instance_baseline.get_data(class_id).expect("baseline data"));
        entity.parse(&mut baseline_br)?;
        entity.parse(br)?;

        self.entities.insert(entity_index, entity);

        Ok(())
    }

    fn handle_update(&mut self, entity_index: usize, br: &mut BitReader) -> Result<()> {
        let entity = self
            .entities
            .get_mut(&entity_index)
            .unwrap_or_else(|| panic!("entity at {}", entity_index));
        entity.parse(br)
    }
}
