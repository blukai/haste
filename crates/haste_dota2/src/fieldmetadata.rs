use crate::{
    dota2_deflat::var_type::{ident_atom, ArrayLength, Decl, IdentAtom},
    fielddecoder::{
        self, BoolDecoder, F32Decoder, FieldDecode, I16Decoder, I32Decoder, I64Decoder, I8Decoder,
        NopDecoder, QAngleDecoder, QuantizedFloatDecoder, StringDecoder, U16Decoder, U32Decoder,
        U64Decoder, U8Decoder, Vector2DDecoder, Vector4DDecoder, VectorDecoder,
    },
    flattenedserializers::FlattenedSerializerField,
};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("unknown array length ident: {0}")]
    UnknownArrayLengthIdent(IdentAtom),
    // crate
    #[error(transparent)]
    FieldDecoder(#[from] fielddecoder::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

// NOTE: Clone is derived because FlattenedSerializerField needs to be clonable.
#[derive(Debug, Clone)]
pub enum FieldSpecialDescriptor {
    // TODO: add element_serializer_name_hash field into Array (to be more
    // correct); probably don't turn FieldMetadata's decoder into Option, but
    // instead create a NopDecoder (with unreachable! in body of decode method)
    Array { length: usize },
    // TODO: add element_serializer_name_hash field into VariableLengthArray (to
    // be more correct) and remove VariableLengthSerializerArray
    VariableLengthArray,
    VariableLengthSerializerArray,
    // TODO: make use of the poiter special type (atm it's useless; but it's
    // supposed to be used to determine whether a new "entity" must be created
    // (and deserialized value of the pointer field (/bool) must not be
    // stored)).
    Pointer,
}

#[derive(Debug, Clone)]
pub struct FieldMetadata {
    pub special_descriptor: Option<FieldSpecialDescriptor>,
    pub decoder: Box<dyn FieldDecode>,
}

impl Default for FieldMetadata {
    #[inline]
    fn default() -> Self {
        Self {
            special_descriptor: None,
            decoder: Box::<NopDecoder>::default(),
        }
    }
}

#[inline]
fn visit_ident(ident: IdentAtom, field: &FlattenedSerializerField) -> Result<FieldMetadata> {
    macro_rules! non_special {
        ($decoder:ident) => {
            Ok(FieldMetadata {
                special_descriptor: None,
                decoder: Box::<$decoder>::default(),
            })
        };
        ($decoder:expr) => {
            Ok(FieldMetadata {
                special_descriptor: None,
                decoder: Box::new($decoder),
            })
        };
    }

    match ident {
        // TODO: smaller decoders (8 and 16 bit)
        // ints
        ident_atom!("int8") => non_special!(I8Decoder),
        ident_atom!("int16") => non_special!(I16Decoder),
        ident_atom!("int32") => non_special!(I32Decoder),
        ident_atom!("int64") => non_special!(I64Decoder),

        // uints
        ident_atom!("uint8") => non_special!(U8Decoder),
        ident_atom!("uint16") => non_special!(U16Decoder),
        ident_atom!("uint32") => non_special!(U32Decoder),
        ident_atom!("uint64") => non_special!(U64Decoder::new(field)),

        // other primitives
        ident_atom!("bool") => non_special!(BoolDecoder),
        ident_atom!("float32") => non_special!(F32Decoder::new(field)?),

        // templates
        ident_atom!("CHandle") => non_special!(U32Decoder),
        ident_atom!("CStrongHandle") => non_special!(U64Decoder::new(field)),

        // pointers (?)
        ident_atom!("CBodyComponent") => Ok(FieldMetadata {
            special_descriptor: Some(FieldSpecialDescriptor::Pointer),
            decoder: Box::<BoolDecoder>::default(),
        }),
        ident_atom!("CLightComponent") => Ok(FieldMetadata {
            special_descriptor: Some(FieldSpecialDescriptor::Pointer),
            decoder: Box::<BoolDecoder>::default(),
        }),
        ident_atom!("CRenderComponent") => Ok(FieldMetadata {
            special_descriptor: Some(FieldSpecialDescriptor::Pointer),
            decoder: Box::<BoolDecoder>::default(),
        }),

        // other custom types
        ident_atom!("CUtlSymbolLarge") => non_special!(StringDecoder),
        ident_atom!("CUtlString") => non_special!(StringDecoder),
        // public/mathlib/vector.h
        ident_atom!("QAngle") => non_special!(QAngleDecoder::new(field)),
        ident_atom!("CNetworkedQuantizedFloat") => non_special!(QuantizedFloatDecoder::new(field)?),
        ident_atom!("GameTime_t") => non_special!(F32Decoder::new(field)?),
        ident_atom!("MatchID_t") => non_special!(U64Decoder::new(field)),
        // public/mathlib/vector.h
        ident_atom!("Vector") => non_special!(VectorDecoder::new(field)?),
        // public/mathlib/vector2d.h
        ident_atom!("Vector2D") => non_special!(Vector2DDecoder::new(field)?),
        // public/mathlib/vector4d.h
        ident_atom!("Vector4D") => non_special!(Vector4DDecoder::new(field)?),
        // game/shared/econ/econ_item_constants.h
        ident_atom!("itemid_t") => non_special!(U64Decoder::new(field)),

        // exceptional specials xd
        ident_atom!("m_SpeechBubbles") => Ok(FieldMetadata {
            special_descriptor: Some(FieldSpecialDescriptor::VariableLengthSerializerArray),
            decoder: Box::<U32Decoder>::default(),
        }),
        // https://github.com/SteamDatabase/GameTracking-CS2/blob/6b3bf6ad44266e3ee4440a0b9b2fee1268812840/game/core/tools/demoinfo2/demoinfo2.txt#L155C83-L155C111
        ident_atom!("DOTA_CombatLogQueryProgress") => Ok(FieldMetadata {
            special_descriptor: Some(FieldSpecialDescriptor::VariableLengthSerializerArray),
            decoder: Box::<U32Decoder>::default(),
        }),

        // ----
        _ => Ok(FieldMetadata {
            special_descriptor: None,
            decoder: Box::<U32Decoder>::default(),
        }),
    }
}

#[inline]
fn visit_template(
    ident: IdentAtom,
    argument: Decl,
    field: &FlattenedSerializerField,
) -> Result<FieldMetadata> {
    match ident {
        ident_atom!("CNetworkUtlVectorBase") => match field.field_serializer_name_hash {
            Some(_) => Ok(FieldMetadata {
                special_descriptor: Some(FieldSpecialDescriptor::VariableLengthSerializerArray),
                decoder: Box::<U32Decoder>::default(),
            }),
            None => visit_any(argument, field).map(|field_metadata| FieldMetadata {
                special_descriptor: Some(FieldSpecialDescriptor::VariableLengthArray),
                decoder: field_metadata.decoder,
            }),
        },
        ident_atom!("CUtlVectorEmbeddedNetworkVar") => Ok(FieldMetadata {
            special_descriptor: Some(FieldSpecialDescriptor::VariableLengthSerializerArray),
            decoder: Box::<U32Decoder>::default(),
        }),
        ident_atom!("CUtlVector") => Ok(FieldMetadata {
            special_descriptor: Some(FieldSpecialDescriptor::VariableLengthSerializerArray),
            decoder: Box::<U32Decoder>::default(),
        }),
        _ => visit_ident(ident, field),
    }
}

#[inline]
fn visit_array(
    decl: Decl,
    length: ArrayLength,
    field: &FlattenedSerializerField,
) -> Result<FieldMetadata> {
    if let Decl::Ident(ref ident) = decl {
        if *ident == ident_atom!("char") {
            return Ok(FieldMetadata {
                special_descriptor: None,
                decoder: Box::<StringDecoder>::default(),
            });
        }
    }

    let length = match length {
        ArrayLength::Ident(ident) => match ident {
            // NOTE: it seems like this was changed from array to vec, see
            // https://github.com/SteamDatabase/GameTracking-CS2/blob/6b3bf6ad44266e3ee4440a0b9b2fee1268812840/game/core/tools/demoinfo2/demoinfo2.txt#L160
            // TODO: test ability draft game
            ident_atom!("MAX_ABILITY_DRAFT_ABILITIES") => Ok(48),
            _ => Err(Error::UnknownArrayLengthIdent(ident.clone())),
        },
        ArrayLength::Number(length) => Ok(length),
    }?;

    visit_any(decl, field).map(|field_metadata| FieldMetadata {
        special_descriptor: Some(FieldSpecialDescriptor::Array { length }),
        decoder: field_metadata.decoder,
    })
}

#[inline]
fn visit_pointer() -> FieldMetadata {
    FieldMetadata {
        special_descriptor: Some(FieldSpecialDescriptor::Pointer),
        decoder: Box::<BoolDecoder>::default(),
    }
}

#[inline]
fn visit_any(decl: Decl, field: &FlattenedSerializerField) -> Result<FieldMetadata> {
    match decl {
        Decl::Ident(ident) => visit_ident(ident, field),
        Decl::Template { ident, argument } => visit_template(ident, *argument, field),
        Decl::Array { decl, length } => visit_array(*decl, length, field),
        Decl::Pointer(_) => Ok(visit_pointer()),
    }
}

pub fn get_field_metadata(
    var_type_decl: Decl,
    field: &FlattenedSerializerField,
) -> Result<FieldMetadata> {
    visit_any(var_type_decl, field)
}

// NOTE: a lot of values are enums, some were discovered in ocratine thing,
// some in kisak-strike, others in
// https://github.com/SteamDatabase/GameTracking-CS2/blob/6b3bf6ad44266e3ee4440a0b9b2fee1268812840/game/core/tools/demoinfo2/demoinfo2.txt#L155C83-L155C111
//
// BeamClipStyle_t // enum
// BeamType_t // enum
// CNetworkUtlVectorBase< CTransform > // public/mathlib/transform.h
// Color // public/color.h
// CourierState_t // enum
// DOTACustomHeroPickRulesPhase_t // enum
// DOTATeam_t // enum
// DOTA_HeroPickState // enum
// DOTA_SHOP_TYPE // enum
// DamageOptions_t // enum
// ERoshanSpawnPhase // enum
// EntityDisolveType_t // enum
// FowBlockerShape_t // enum
// MoveCollide_t // enum
// MoveType_t // enum
// PingConfirmationIconType // enum
// PlayerConnectedState // enum
// PointWorldTextJustifyHorizontal_t // enum
// PointWorldTextJustifyVertical_t // enum
// PointWorldTextReorientMode_t // enum
// RenderFx_t // enum
// RenderMode_t // enum
// ShopItemViewMode_t // enum
// SolidType_t // enum
// SurroundingBoundsType_t // enum
// ValueRemapperHapticsType_t // enum
// ValueRemapperInputType_t // enum
// ValueRemapperMomentumType_t // enum
// ValueRemapperOutputType_t // enum
// ValueRemapperRatchetType_t // enum
// WeaponState_t // enum
// attrib_definition_index_t // game/shared/econ/econ_item_constants.h
// attributeprovidertypes_t // game/shared/econ/attribute_manager.h
// item_definition_index_t // game/shared/econ/econ_item_constants.h
// itemid_t // game/shared/econ/econ_item_constants.h
// style_index_t // game/shared/econ/econ_item_constants.h
