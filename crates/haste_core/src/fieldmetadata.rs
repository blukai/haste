use crate::{
    fielddecoder::{self, FieldDecoder},
    flattenedserializers::FlattenedSerializerField,
    vartype::{Expr, Lit},
};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("unknown array length ident: {0}")]
    UnknownArrayLenIdent(String),
    // crate
    #[error(transparent)]
    FieldDecoder(#[from] fielddecoder::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

// NOTE: Clone is derived because FlattenedSerializerField needs to be clonable.
#[derive(Debug, Clone)]
pub enum FieldSpecialDescriptor {
    FixedArray {
        length: usize,
    },

    /// this variant differs from [FieldSpecialDescriptor::DynamicSerializerArray] in that it can
    /// contain primitive values (e.g., u8, bool) and more complex types (e.g., Vector4D, Vector),
    /// but it can not contain other serializers.
    ///
    /// example entity fields:
    /// ```txt
    /// m_PathNodes_Position: CNetworkUtlVectorBase< Vector > = 2
    /// m_PathNodes_Position.0: Vector = [0.0, 0.0, 0.0]
    /// m_PathNodes_Position.1: Vector = [-736.0029, 596.5974, 384.0]
    /// ```
    DynamicArray {
        /// decoder for the items of the dynamic array.
        ///
        /// decoder must be capable of decoding the type specified in the array's generic argument.
        /// for example, if the var type is `CNetworkUtlVectorBase< Vector >`, the decoder must be
        /// able to decode `Vector` values.
        decoder: FieldDecoder,
    },

    /// represents a dynamic array of fields that must be deserialized by the serializer specified
    /// by `field_serializer_name`.
    ///
    /// this variant differs from [FieldSpecialDescriptor::DynamicArray] in that it houses other
    /// serializers.
    ///
    /// example entity fields:
    /// ```txt
    /// m_vecStatViewerModifierValues: CUtlVectorEmbeddedNetworkVar< StatViewerModifierValues_t > = 2
    /// m_vecStatViewerModifierValues.0.m_SourceModifierID: CUtlStringToken = 1058891786
    /// m_vecStatViewerModifierValues.0.m_eValType: EModifierValue = 11
    /// m_vecStatViewerModifierValues.0.m_flValue: float32 = 3.0
    /// m_vecStatViewerModifierValues.1.m_SourceModifierID: CUtlStringToken = 2201601853
    /// m_vecStatViewerModifierValues.1.m_eValType: EModifierValue = 161
    /// m_vecStatViewerModifierValues.1.m_flValue: float32 = 2.0
    /// ```
    DynamicSerializerArray,

    // TODO: make use of the poiter special type (atm it's useless; but it's
    // supposed to be used to determine whether a new "entity" must be created
    // (and deserialized value of the pointer field (/bool) must not be
    // stored)).
    Pointer,
}

impl FieldSpecialDescriptor {
    #[inline(always)]
    pub(crate) fn is_dynamic_array(&self) -> bool {
        matches!(
            self,
            Self::DynamicArray { .. } | Self::DynamicSerializerArray
        )
    }
}

#[derive(Debug, Clone)]
pub struct FieldMetadata {
    pub special_descriptor: Option<FieldSpecialDescriptor>,
    pub decoder: FieldDecoder,
}

impl Default for FieldMetadata {
    #[inline]
    fn default() -> Self {
        Self {
            special_descriptor: None,
            decoder: fielddecoder::decode_invalid,
        }
    }
}

#[inline]
fn visit_ident(ident: &str, field: &FlattenedSerializerField) -> Result<FieldMetadata> {
    macro_rules! non_special {
        ($decoder:expr) => {
            Ok(FieldMetadata {
                special_descriptor: None,
                decoder: $decoder,
            })
        };
    }

    macro_rules! pointer {
        () => {
            Ok(FieldMetadata {
                special_descriptor: Some(FieldSpecialDescriptor::Pointer),
                decoder: fielddecoder::decode_bool,
            })
        };
    }

    match ident {
        // TODO: (very maybe) smaller decoders (8 and 16 bit). but in terms for decode speed the
        // impact is less then negligible.

        // ints
        "int8" => non_special!(fielddecoder::decode_u32),
        "int16" => non_special!(fielddecoder::decode_u32),
        "int32" => non_special!(fielddecoder::decode_u32),
        "int64" => non_special!(fielddecoder::determine_u64_decoder(field)),

        // uints
        "uint8" => non_special!(fielddecoder::decode_u32),
        "uint16" => non_special!(fielddecoder::decode_u32),
        "uint32" => non_special!(fielddecoder::decode_u32),
        "uint64" => non_special!(fielddecoder::determine_u64_decoder(field)),

        // other primitives
        "bool" => non_special!(fielddecoder::decode_bool),
        "float32" => non_special!(fielddecoder::determine_f32_decoder(field)),

        // templates
        "CHandle" => non_special!(fielddecoder::decode_u32),
        "CStrongHandle" => non_special!(fielddecoder::determine_u64_decoder(field)),

        // pointers (?)
        // https://github.com/SteamDatabase/GameTracking-Deadlock/blob/master/game/core/tools/demoinfo2/demoinfo2.txt#L130
        "CBodyComponentDCGBaseAnimating" => pointer!(),
        "CBodyComponentBaseAnimating" => pointer!(),
        "CBodyComponentBaseAnimatingOverlay" => pointer!(),
        "CBodyComponentBaseModelEntity" => pointer!(),
        "CBodyComponent" => pointer!(),
        "CBodyComponentSkeletonInstance" => pointer!(),
        "CBodyComponentPoint" => pointer!(),
        "CLightComponent" => pointer!(),
        "CRenderComponent" => pointer!(),
        // https://github.com/SteamDatabase/GameTracking-Deadlock/blob/1e09d0e1289914e776b8d5783834478782a67468/game/core/pak01_dir/scripts/replay_compatability_settings.txt#L56
        "C_BodyComponentBaseAnimating" => pointer!(),
        "C_BodyComponentBaseAnimatingOverlay" => pointer!(),
        "CPhysicsComponent" => pointer!(),

        // other custom types
        "CUtlSymbolLarge" => non_special!(fielddecoder::decode_string),
        "CUtlString" => non_special!(fielddecoder::decode_string),
        // public/mathlib/vector.h
        "QAngle" => non_special!(fielddecoder::determine_qangle_decoder(field)),
        // NOTE: not all quantized floats are actually quantized (if bit_count is 0 or 32 it's
        // not!) F32Decoder will determine which kind of f32 decoder to use.
        //
        // TODO(blukai): fix up quantized float decoder so that it can handle 0 or 32 bit counts.
        "CNetworkedQuantizedFloat" => non_special!(fielddecoder::determine_f32_decoder(field)),
        "GameTime_t" => non_special!(fielddecoder::determine_f32_decoder(field)),
        "MatchID_t" => non_special!(fielddecoder::determine_u64_decoder(field)),
        // public/mathlib/vector.h
        "Vector" => non_special!(fielddecoder::determine_vector_decoder(field)),
        // public/mathlib/vector2d.h
        "Vector2D" => non_special!(fielddecoder::decode_vector2d),
        // public/mathlib/vector4d.h
        "Vector4D" => non_special!(fielddecoder::decode_vector4d),
        // game/shared/econ/econ_item_constants.h
        "itemid_t" => non_special!(fielddecoder::determine_u64_decoder(field)),
        "HeroFacetKey_t" => non_special!(fielddecoder::determine_u64_decoder(field)),
        "BloodType" => non_special!(fielddecoder::decode_u32),

        // exceptional specials xd
        "m_SpeechBubbles" => Ok(FieldMetadata {
            special_descriptor: Some(FieldSpecialDescriptor::DynamicSerializerArray),
            decoder: fielddecoder::decode_u32,
        }),
        // https://github.com/SteamDatabase/GameTracking-CS2/blob/6b3bf6ad44266e3ee4440a0b9b2fee1268812840/game/core/tools/demoinfo2/demoinfo2.txt#L155C83-L155C111
        "DOTA_CombatLogQueryProgress" => Ok(FieldMetadata {
            special_descriptor: Some(FieldSpecialDescriptor::DynamicSerializerArray),
            decoder: fielddecoder::decode_u32,
        }),

        // ----
        _ => Ok(FieldMetadata {
            special_descriptor: None,
            decoder: fielddecoder::decode_u32,
        }),
    }
}

#[inline]
fn visit_template(
    expr: Expr,
    arg: Expr,
    field: &FlattenedSerializerField,
) -> Result<FieldMetadata> {
    let Expr::Ident(ident) = expr else {
        unreachable!();
    };

    if matches!(
        ident,
        "CNetworkUtlVectorBase" | "CUtlVectorEmbeddedNetworkVar" | "CUtlVector"
    ) {
        if field.field_serializer_name.is_some() {
            return Ok(FieldMetadata {
                special_descriptor: Some(FieldSpecialDescriptor::DynamicSerializerArray),
                decoder: fielddecoder::decode_u32,
            });
        }

        return visit_any(arg, field).map(|field_metadata| FieldMetadata {
            special_descriptor: Some(FieldSpecialDescriptor::DynamicArray {
                decoder: field_metadata.decoder,
            }),
            decoder: fielddecoder::decode_u32,
        });
    }

    return visit_ident(ident, field);
}

#[inline]
fn visit_array(expr: Expr, len: Expr, field: &FlattenedSerializerField) -> Result<FieldMetadata> {
    if let Expr::Ident(ident) = expr {
        if ident == "char" {
            return Ok(FieldMetadata {
                special_descriptor: None,
                decoder: fielddecoder::decode_string,
            });
        }
    }

    let length = match len {
        Expr::Ident(ident) => match ident {
            // NOTE: it seems like this was changed from array to vec, see
            // https://github.com/SteamDatabase/GameTracking-CS2/blob/6b3bf6ad44266e3ee4440a0b9b2fee1268812840/game/core/tools/demoinfo2/demoinfo2.txt#L160
            // TODO: test ability draft game
            "MAX_ABILITY_DRAFT_ABILITIES" => Ok(48),
            _ => Err(Error::UnknownArrayLenIdent(ident.to_owned())),
        },
        Expr::Lit(Lit::Num(length)) => Ok(length),
        _ => unreachable!(),
    }?;

    visit_any(expr, field).map(|field_metadata| FieldMetadata {
        special_descriptor: Some(FieldSpecialDescriptor::FixedArray { length }),
        decoder: field_metadata.decoder,
    })
}

#[inline]
fn visit_pointer() -> Result<FieldMetadata> {
    Ok(FieldMetadata {
        special_descriptor: Some(FieldSpecialDescriptor::Pointer),
        decoder: fielddecoder::decode_bool,
    })
}

#[inline]
fn visit_any(expr: Expr, field: &FlattenedSerializerField) -> Result<FieldMetadata> {
    match expr {
        Expr::Ident(ident) => visit_ident(ident, field),
        Expr::Template { expr, arg } => visit_template(*expr, *arg, field),
        Expr::Array { expr, len } => visit_array(*expr, *len, field),
        Expr::Pointer(_) => visit_pointer(),
        _ => unreachable!(),
    }
}

pub fn get_field_metadata(expr: Expr, field: &FlattenedSerializerField) -> Result<FieldMetadata> {
    visit_any(expr, field)
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
