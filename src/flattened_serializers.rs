use crate::{
    bitreader::BitReader,
    error::{required, Result},
    field_prop::{self, FieldProp},
    protos,
    varint::VarIntRead,
};
use compact_str::CompactString;
use hashbrown::{hash_map::DefaultHashBuilder, HashMap};
use prost::Message;
use std::{alloc::Allocator, fmt::Debug, io::Cursor, rc::Rc};

// some info about string tables (outdated in a way, but still nice).
// https://developer.valvesoftware.com/wiki/Networking_Events_%26_Messages
// https://developer.valvesoftware.com/wiki/Networking_Entities

// csgo: engine/dt_recv_eng.cpp, engine/serializedentity.cpp

pub type FieldDecoderFn<A> = fn(&mut BitReader, &FlattenedSerializerField<A>) -> Result<FieldProp>;
pub struct FieldDecoder<A: Allocator + Clone + Debug>(FieldDecoderFn<A>);

impl<A: Allocator + Clone + Debug> core::fmt::Debug for FieldDecoder<A> {
    fn fmt<'a>(&'a self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("FieldDecoder").finish()
    }
}

impl<A: Allocator + Clone + Debug> std::ops::Deref for FieldDecoder<A> {
    type Target = FieldDecoderFn<A>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct FlattenedSerializerField<A: Allocator + Clone + Debug> {
    // TODO: figure out which fields should, and which should not be pub
    pub var_type: CompactString,
    pub var_name: CompactString,

    // TODO: figure out which fields should, and which should not be optional
    pub bit_count: Option<i32>,
    pub low_value: Option<f32>,
    pub high_value: Option<f32>,
    pub encode_flags: Option<i32>,
    field_serializer_name: Option<CompactString>,
    field_serializer_version: Option<i32>,
    send_node: Option<CompactString>,
    pub var_encoder: Option<CompactString>,

    // custom fields
    pub field_serializer: Option<Rc<FlattenedSerializer<A>>>,
    pub decoder: Option<FieldDecoder<A>>,
    pub size: usize,
    pub is_dynamic: bool,
}

impl<A: Allocator + Clone + Debug> FlattenedSerializerField<A> {
    fn new(
        msg: &protos::CsvcMsgFlattenedSerializer,
        field: &protos::ProtoFlattenedSerializerFieldT,
    ) -> Self {
        let resolve_string = |v: i32| Some(CompactString::from(&msg.symbols[v as usize]));

        Self {
            // NOTE: it seems like these field are alwyas Some, and never None!
            var_type: field
                .var_type_sym
                .and_then(resolve_string)
                .expect("var_type"),
            var_name: field
                .var_name_sym
                .and_then(resolve_string)
                .expect("var_name"),

            bit_count: field.bit_count,
            low_value: field.low_value,
            high_value: field.high_value,
            encode_flags: field.encode_flags,
            field_serializer_name: field.field_serializer_name_sym.and_then(resolve_string),
            field_serializer_version: field.field_serializer_version,
            send_node: field.send_node_sym.and_then(resolve_string),
            var_encoder: field.var_encoder_sym.and_then(resolve_string),

            field_serializer: None,
            decoder: None,
            size: 0,
            is_dynamic: false,
        }
    }
}

#[derive(Debug)]
pub struct FlattenedSerializer<A: Allocator + Clone + Debug> {
    pub serializer_name: CompactString,
    pub serializer_version: i32,
    pub fields: Vec<Rc<FlattenedSerializerField<A>>, A>,
}

impl<A: Allocator + Clone + Debug> FlattenedSerializer<A> {
    fn new_in(
        msg: &protos::CsvcMsgFlattenedSerializer,
        fs: &protos::ProtoFlattenedSerializerT,
        alloc: A,
    ) -> Result<Self> {
        Ok(Self {
            serializer_name: CompactString::from(
                &msg.symbols[fs.serializer_name_sym.ok_or(required!())? as usize],
            ),
            serializer_version: fs.serializer_version.ok_or(required!())?,
            fields: Vec::with_capacity_in(fs.fields_index.len(), alloc.clone()),
        })
    }
}

type FieldMap<A> = HashMap<i32, Rc<FlattenedSerializerField<A>>, DefaultHashBuilder, A>;
type SerializerMap<A> = HashMap<CompactString, Rc<FlattenedSerializer<A>>, DefaultHashBuilder, A>;

pub struct FlattenedSerializers<A: Allocator + Clone + Debug> {
    serializer_map: SerializerMap<A>,
}

impl<A: Allocator + Clone + Debug> FlattenedSerializers<A> {
    pub fn new_in(proto: protos::CDemoSendTables, alloc: A) -> Result<Self> {
        let data = proto.data.ok_or(required!())?;

        let mut data_cursored = Cursor::new(&data);
        let size = data_cursored.read_varu32()? as usize;
        let offset = data_cursored.position() as usize;

        let msg = protos::CsvcMsgFlattenedSerializer::decode(&data[offset..offset + size])?;

        let mut field_map: FieldMap<A> = FieldMap::new_in(alloc.clone());
        let mut serializer_map: SerializerMap<A> = SerializerMap::new_in(alloc.clone());

        for fs in msg.serializers.iter() {
            let mut serializer = FlattenedSerializer::new_in(&msg, fs, alloc.clone())?;

            for field_index in fs.fields_index.iter() {
                if !field_map.contains_key(field_index) {
                    let mut field =
                        FlattenedSerializerField::new(&msg, &msg.fields[*field_index as usize]);

                    // ----

                    // TODO: MACRO!

                    match field.var_type.as_str() {
                        "float32" | "GameTime_t" => match field.var_name.as_str() {
                            "m_flSimulationTime" | "m_flAnimTime" => {
                                field.decoder = Some(FieldDecoder(field_prop::decode_simulation_time));
                            }
                            _ => {
                                field.decoder = Some(FieldDecoder(field_prop::decode_float32));
                            },
                        },

                        // csgo does the following for CHandle
                        // RecvPropEHandle( RECVINFO(m_hOwnerEntity) ),
                        // RecvProp RecvPropEHandle(
                        //     const char *pVarName,
                        //     int offset,
                        //     int sizeofVar,
                        //     RecvVarProxyFn proxyFn )
                        // {
                        //     return RecvPropInt( pVarName, offset, sizeofVar, 0, proxyFn );
                        // }
                        //
                        // NOTE: there are no flags defined for EHandle, so let's treat it
                        // as unsigned (otherwise there must be a flag SPROP_UNSIGNED).
                        //
                        // more info on CHandle:
                        // https://developer.valvesoftware.com/wiki/CHandle
                        // > CHandle is a C++ class that represents a 32-bit ID (entindex +
                        // serial number) unique to every past and present entity in a game.
                        "CHandle< CBaseEntity >"
                        | "uint16"
                        | "CGameSceneNodeHandle"
                        | "CUtlStringToken"
                        | "uint8"
                        | "DamageOptions_t"
                        | "MoveCollide_t"
                        | "MoveType_t"
                        | "uint32"
                        | "RenderMode_t"
                        | "RenderFx_t"
                        | "Color"
                        | "SolidType_t"
                        | "SurroundingBoundsType_t"
                        // for the type below i'm just assuming clarity's default
                        // decoder (DEFAULT_DECODER = new IntVarUnsignedDecoder)
                        | "CUtlVectorEmbeddedNetworkVar< EntityRenderAttribute_t >"
                        | "CHandle< CBasePlayerPawn >"
                        | "PlayerConnectedState"
                        | "PlayerID_t"
                        | "ShopItemViewMode_t"
                        | "GameTick_t"
                        | "CHandle< CBasePlayerController >" => {
                            field.decoder = Some(FieldDecoder(field_prop::decode_varu32));
                        },

                        // why CBodyComponent, CEntityIdentity*, etc.. are bool? idk, it's
                        // almost 4am, maybe i'll dig more into this tomorrow...
                        "CBodyComponent" | "bool" | "CEntityIdentity*" | "CRenderComponent" => {
                            field.decoder = Some(FieldDecoder(field_prop::decode_bool));
                        }

                        "CNetworkedQuantizedFloat" => {
                            field.decoder = Some(FieldDecoder(field_prop::decode_quantized_float));
                        },
                        "QAngle" => {
                            field.decoder = Some(FieldDecoder(field_prop::decode_qangle));
                        },
                        "CStrongHandle< InfoForResourceTypeCModel >" | "uint64" => {
                            field.decoder = Some(FieldDecoder(field_prop::decode_varu64));
                        }
                        "int8" | "int32" => {
                            field.decoder = Some(FieldDecoder(field_prop::decode_vari32));
                        },
                        "Vector" => {
                            field.decoder = Some(FieldDecoder(field_prop::decode_vector));
                        },

                        "CNetworkUtlVectorBase< CHandle< CBaseModelEntity > >"
                        | "CNetworkUtlVectorBase< CHandle< CBasePlayerController > >"
                        | "CNetworkUtlVectorBase< CHandle< CBasePlayerPawn > >" => {
                            field.decoder = Some(FieldDecoder(field_prop::decode_varu32));
                            field.is_dynamic = true;
                        }

                        "uint32[1]" => {
                            field.decoder = Some(FieldDecoder(field_prop::decode_varu32));
                            field.size = 1;
                        }
                        "char[128]" => {
                            field.decoder = Some(FieldDecoder(field_prop::decode_string));
                            field.size = 128;
                        }
                        "float32[3]" => {
                            field.decoder = Some(FieldDecoder(field_prop::decode_float32));
                            field.size = 3;
                        }
                        "int32[4]" => {
                            field.decoder = Some(FieldDecoder(field_prop::decode_vari32));
                            field.size = 4;
                        }
                        "int32[2]" => {
                            field.decoder = Some(FieldDecoder(field_prop::decode_vari32));
                            field.size = 2;
                        }
                        "char[64]" => {
                            field.decoder = Some(FieldDecoder(field_prop::decode_string));
                            field.size = 64;
                        }
                        "AbilityID_t[9]" => {
                            field.decoder = Some(FieldDecoder(field_prop::decode_varu32));
                            field.size = 9;
                        }
                        "bool[9]" => {
                            field.decoder = Some(FieldDecoder(field_prop::decode_bool));
                            field.size = 9;
                        }
                        "char[129]" => {
                            field.decoder = Some(FieldDecoder(field_prop::decode_string));
                            field.size = 129;
                        }
                        "char[33]" => {
                            field.decoder = Some(FieldDecoder(field_prop::decode_string));
                            field.size = 33;
                        }

                        _ => {},
                    };

                    if let Some(ve) = field.var_encoder.as_ref() {
                        match ve.as_str() {
                            "qangle" => {
                                field.decoder = Some(FieldDecoder(field_prop::decode_qangle));
                            }
                            "coord" => {
                                field.decoder =
                                    Some(FieldDecoder(field_prop::decode_float32_coord));
                            }
                            "fixed64" => {
                                field.decoder = Some(FieldDecoder(field_prop::decode_fixed64));
                            }
                            _ => {}
                        };
                    }

                    field.field_serializer = field
                        .field_serializer_name
                        .as_ref()
                        .and_then(|fsn| serializer_map.get(fsn).map(|rc| rc.clone()));

                    // ----

                    field_map.insert(*field_index, Rc::new(field));
                }
                // NOTE: unwrap is safe because we just inserted field above
                let field = field_map.get(field_index).unwrap();

                serializer.fields.push(field.clone());
            }

            // TODO: handle serializer version
            serializer_map.insert(serializer.serializer_name.clone(), Rc::new(serializer));
        }

        Ok(Self { serializer_map })
    }

    pub fn get_by_class_id(&self, key: &str) -> Option<&FlattenedSerializer<A>> {
        self.serializer_map.get(key).map(|rc| rc.as_ref())
    }
}
