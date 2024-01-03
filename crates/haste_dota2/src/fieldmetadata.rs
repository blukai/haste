use haste_dota2_deflat::var_type::{ident_atom, ArrayLength, IdentAtom, TypeDecl};

use crate::{
    fielddecoder::{
        self, BoolDecoder, F32Decoder, FieldDecode, I32Decoder, I64Decoder, QAngleDecoder,
        QuantizedFloatDecoder, StringDecoder, U32Decoder, U64Decoder, Vec2Decoder, Vec3Decoder,
        Vec4Decoder,
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

#[inline]
fn handle_ident(ident: &IdentAtom, field: &FlattenedSerializerField) -> Result<FieldMetadata> {
    macro_rules! unspecial {
        ($decoder:expr) => {
            Ok(FieldMetadata {
                special_descriptor: None,
                decoder: Box::new($decoder),
            })
        };
    }

    match *ident {
        // TODO: smaller decoders (8 and 16 bit)
        // ints
        ident_atom!("int8") => unspecial!(I32Decoder::default()),
        ident_atom!("int16") => unspecial!(I32Decoder::default()),
        ident_atom!("int32") => unspecial!(I32Decoder::default()),
        ident_atom!("int64") => unspecial!(I64Decoder::default()),
        // uints
        ident_atom!("uint8") => unspecial!(U32Decoder::default()),
        ident_atom!("uint16") => unspecial!(U32Decoder::default()),
        ident_atom!("uint32") => unspecial!(U32Decoder::default()),
        ident_atom!("uint64") => unspecial!(U64Decoder::new(field)),

        // other primitives
        ident_atom!("bool") => unspecial!(BoolDecoder::default()),
        ident_atom!("float32") => unspecial!(F32Decoder::new(field)?),
        ident_atom!("char") => unspecial!(StringDecoder::default()),

        // templates
        ident_atom!("CHandle") => unspecial!(U32Decoder::default()),
        ident_atom!("CStrongHandle") => unspecial!(U64Decoder::new(field)),

        // pointers (?)
        ident_atom!("CBodyComponent") => Ok(FieldMetadata {
            special_descriptor: Some(FieldSpecialDescriptor::Pointer),
            decoder: Box::new(BoolDecoder::default()),
        }),
        ident_atom!("CLightComponent") => Ok(FieldMetadata {
            special_descriptor: Some(FieldSpecialDescriptor::Pointer),
            decoder: Box::new(BoolDecoder::default()),
        }),
        ident_atom!("CRenderComponent") => Ok(FieldMetadata {
            special_descriptor: Some(FieldSpecialDescriptor::Pointer),
            decoder: Box::new(BoolDecoder::default()),
        }),

        // other custom types
        ident_atom!("CUtlSymbolLarge") => unspecial!(StringDecoder::default()),
        ident_atom!("CUtlString") => unspecial!(StringDecoder::default()),
        ident_atom!("QAngle") => unspecial!(QAngleDecoder::new(field)),
        ident_atom!("Vector") => unspecial!(Vec3Decoder::new(field)?),
        ident_atom!("CNetworkedQuantizedFloat") => unspecial!(QuantizedFloatDecoder::new(field)?),
        ident_atom!("GameTime_t") => unspecial!(F32Decoder::new(field)?),
        ident_atom!("MatchID_t") => unspecial!(U64Decoder::new(field)),
        ident_atom!("Vector2D") => unspecial!(Vec2Decoder::new(field)?),
        ident_atom!("Vector4D") => unspecial!(Vec4Decoder::new(field)?),
        // game/shared/econ/econ_item_constants.h
        ident_atom!("itemid_t") => unspecial!(U64Decoder::new(field)),

        // exceptional specials xd
        ident_atom!("m_SpeechBubbles") => Ok(FieldMetadata {
            special_descriptor: Some(FieldSpecialDescriptor::VariableLengthSerializerArray),
            decoder: Box::new(U32Decoder::default()),
        }),
        // https://github.com/SteamDatabase/GameTracking-CS2/blob/6b3bf6ad44266e3ee4440a0b9b2fee1268812840/game/core/tools/demoinfo2/demoinfo2.txt#L155C83-L155C111
        ident_atom!("DOTA_CombatLogQueryProgress") => Ok(FieldMetadata {
            special_descriptor: Some(FieldSpecialDescriptor::VariableLengthSerializerArray),
            decoder: Box::new(U32Decoder::default()),
        }),

        // ----
        _ => Ok(FieldMetadata {
            special_descriptor: None,
            decoder: Box::new(U32Decoder::default()),
        }),
    }
}

#[inline]
fn handle_template(
    ident: &IdentAtom,
    argument: &TypeDecl,
    field: &FlattenedSerializerField,
) -> Result<FieldMetadata> {
    match *ident {
        ident_atom!("CNetworkUtlVectorBase") => match field.field_serializer_name_hash {
            Some(_) => Ok(FieldMetadata {
                special_descriptor: Some(FieldSpecialDescriptor::VariableLengthSerializerArray),
                decoder: Box::new(U32Decoder::default()),
            }),
            None => handle_any(argument, field).map(|field_metadata| FieldMetadata {
                special_descriptor: Some(FieldSpecialDescriptor::VariableLengthArray),
                decoder: field_metadata.decoder,
            }),
        },
        ident_atom!("CUtlVectorEmbeddedNetworkVar") => Ok(FieldMetadata {
            special_descriptor: Some(FieldSpecialDescriptor::VariableLengthSerializerArray),
            decoder: Box::new(U32Decoder::default()),
        }),
        ident_atom!("CUtlVector") => Ok(FieldMetadata {
            special_descriptor: Some(FieldSpecialDescriptor::VariableLengthSerializerArray),
            decoder: Box::new(U32Decoder::default()),
        }),
        _ => handle_ident(ident, field),
    }
}

#[inline]
fn handle_array(
    type_decl: &TypeDecl,
    length: &ArrayLength,
    field: &FlattenedSerializerField,
) -> Result<FieldMetadata> {
    let length = match length {
        ArrayLength::Ident(ident) => match *ident {
            ident_atom!("MAX_ABILITY_DRAFT_ABILITIES") => Ok(48),
            _ => Err(Error::UnknownArrayLengthIdent(ident.clone())),
        },
        ArrayLength::Number(length) => Ok(*length),
    }?;

    handle_any(type_decl, field).map(|field_metadata| FieldMetadata {
        special_descriptor: Some(FieldSpecialDescriptor::Array { length }),
        decoder: field_metadata.decoder,
    })
}

#[inline]
fn handle_pointer() -> FieldMetadata {
    FieldMetadata {
        special_descriptor: Some(FieldSpecialDescriptor::Pointer),
        decoder: Box::new(BoolDecoder::default()),
    }
}

#[inline]
fn handle_any(type_decl: &TypeDecl, field: &FlattenedSerializerField) -> Result<FieldMetadata> {
    match type_decl {
        TypeDecl::Ident(ident) => handle_ident(ident, field),
        TypeDecl::Template { ident, argument } => handle_template(ident, argument, field),
        TypeDecl::Array { type_decl, length } => handle_array(type_decl, length, field),
        TypeDecl::Pointer(_) => Ok(handle_pointer()),
    }
}

pub fn get_field_metadata(field: &FlattenedSerializerField) -> Result<FieldMetadata> {
    handle_any(&field.type_decl, field)
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
