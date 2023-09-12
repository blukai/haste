use crate::{
    bitbuf::{self, BitReader},
    entityclasses::EntityClasses,
    fielddecoder,
    fieldpath::{self},
    fieldvalue::FieldValue,
    flattenedserializers::{FlattenedSerializer, FlattenedSerializers},
    hashers::{I32HashBuilder, U64HashBuiler},
    instancebaseline::InstanceBaseline,
    protos,
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
pub struct Entity {
    flattened_serializer: Rc<FlattenedSerializer>,
    field_values: HashMap<u64, FieldValue, U64HashBuiler>,
}

impl Entity {
    fn parse(&mut self, br: &mut BitReader) -> Result<()> {
        // eprintln!("-- {}", self.flattened_serializer.serializer_name);

        fieldpath::FIELD_PATHS.with(|fps| {
            let mut fps = fps.borrow_mut();
            let fps = fieldpath::read_field_paths(br, &mut fps)?;
            for fp in fps {
                let field = match fp.position {
                    0 => self.flattened_serializer.get_child(fp.get(0)),
                    1 => self
                        .flattened_serializer
                        .get_child(fp.get(0))
                        .get_child(fp.get(1)),
                    2 => self
                        .flattened_serializer
                        .get_child(fp.get(0))
                        .get_child(fp.get(1))
                        .get_child(fp.get(2)),
                    3 => self
                        .flattened_serializer
                        .get_child(fp.get(0))
                        .get_child(fp.get(1))
                        .get_child(fp.get(2))
                        .get_child(fp.get(3)),
                    4 => self
                        .flattened_serializer
                        .get_child(fp.get(0))
                        .get_child(fp.get(1))
                        .get_child(fp.get(2))
                        .get_child(fp.get(3))
                        .get_child(fp.get(4)),
                    5 => self
                        .flattened_serializer
                        .get_child(fp.get(0))
                        .get_child(fp.get(1))
                        .get_child(fp.get(2))
                        .get_child(fp.get(3))
                        .get_child(fp.get(4))
                        .get_child(fp.get(5)),
                    6 => self
                        .flattened_serializer
                        .get_child(fp.get(0))
                        .get_child(fp.get(1))
                        .get_child(fp.get(2))
                        .get_child(fp.get(3))
                        .get_child(fp.get(4))
                        .get_child(fp.get(5))
                        .get_child(fp.get(6)),
                    _ => panic!("invalid position: {}", fp.position),
                };

                // eprint!(
                //     "{:?} {} {} ",
                //     &fp.data[..=fp.position],
                //     field.var_name,
                //     field.var_type
                // );

                // SAFETY: metadata for the field is generated in
                // flattenedserializers.rs; if metadata cannot be generated -
                // FlattenedSerializers::parse will fail thus we'll never get
                // here. it is safe to assume that field metadata cannot be
                // None.
                let field_metadata = unsafe { field.metadata.as_ref().unwrap_unchecked() };
                let field_value = field_metadata.decoder.decode(br)?;

                // eprintln!(" -> {:?}", field_value);

                self.field_values.insert(field.var_name_hash, field_value);
            }

            Ok(())
        })
    }
}

pub struct Entities {
    // TODO: use Vec<Option<.. or Vec<MaybeUninit<.. or implement NoOpHasher
    // because keys are indexes, see
    // https://sourcegraph.com/github.com/actix/actix-web@d8df60bf4c04c3cbb99bcf19a141c202223e07ea/-/blob/actix-http/src/extensions.rs?L13
    entities: HashMap<i32, Entity, I32HashBuilder>,
}

impl Default for Entities {
    fn default() -> Self {
        Self {
            entities: HashMap::with_capacity_and_hasher(4096, I32HashBuilder::default()),
        }
    }
}

impl Entities {
    pub fn read_packet_entities(
        &mut self,
        svcmsg: protos::CsvcMsgPacketEntities,
        entity_classes: &EntityClasses,
        instance_baseline: &InstanceBaseline,
        flattened_serializers: &FlattenedSerializers,
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
                        entidx,
                        &mut br,
                        entity_classes,
                        instance_baseline,
                        flattened_serializers,
                    )?;
                }
                UpdateType::LeavePVS => {
                    if (update_flags & FHDR_DELETE) != 0 {
                        self.entities.remove(&(entidx));
                    }
                }
                UpdateType::DeltaEnt => {
                    self.handle_update(entidx, &mut br)?;
                }
            }
        }

        Ok(())
    }

    fn handle_create(
        &mut self,
        entidx: i32,
        br: &mut BitReader,
        entity_classes: &EntityClasses,
        instance_baseline: &InstanceBaseline,
        flattened_serializers: &FlattenedSerializers,
    ) -> Result<()> {
        let class_id = br.read_ubitlong(entity_classes.bits)? as i32;
        let _serial = br.read_ubitlong(17)?;
        let _unknown = br.read_uvarint32()?;

        let class_info = entity_classes.get_by_id(class_id).expect("class info");
        let flattened_serializer = flattened_serializers
            .get_by_serializer_name_hash(class_info.network_name_hash)
            .expect("flattened serializer")
            .clone();

        let field_values = HashMap::with_capacity_and_hasher(
            flattened_serializer.fields.len(),
            U64HashBuiler::default(),
        );

        let mut entity = Entity {
            flattened_serializer,
            field_values,
        };

        let baseline_data = instance_baseline.get_data(class_id).expect("baseline data");
        let mut baseline_br = BitReader::new(baseline_data.as_bytes());
        entity.parse(&mut baseline_br)?;
        entity.parse(br)?;

        self.entities.insert(entidx, entity);

        Ok(())
    }

    fn handle_update(&mut self, entidx: i32, br: &mut BitReader) -> Result<()> {
        // SAFETY: if entity was ever created, and not deleted, it can be
        // updated!
        let entity = unsafe { self.entities.get_mut(&entidx).unwrap_unchecked() };
        entity.parse(br)
    }
}
