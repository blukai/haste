use crate::{
    fielddecoder::{
        self, BoolDecoder, F32Decoder, FieldDecode, I16Decoder, I32Decoder, I64Decoder, I8Decoder,
        NopDecoder, QAngleDecoder, QuantizedFloatDecoder, StringDecoder, U16Decoder, U32Decoder,
        U64Decoder, U8Decoder, Vector2DDecoder, Vector4DDecoder, VectorDecoder,
    },
    flattenedserializers::{FlattenedSerializerContext, FlattenedSerializerField},
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

    /// this variant differs from [`FieldSpecialDescriptor::DynamicSerializerArray`] in that it can
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
        decoder: Box<dyn FieldDecode>,
    },

    /// represents a dynamic array of fields that must be deserialized by the serializer specified
    /// by `field_serializer_name`.
    ///
    /// this variant differs from [`FieldSpecialDescriptor::DynamicArray`] in that it houses other
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
fn visit_ident(
    ident: &str,
    field: &FlattenedSerializerField,
    ctx: &FlattenedSerializerContext,
) -> Result<FieldMetadata> {
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
        "int8" => non_special!(I8Decoder),
        "int16" => non_special!(I16Decoder),
        "int32" => non_special!(I32Decoder),
        "int64" => non_special!(I64Decoder),

        // uints
        "uint8" => non_special!(U8Decoder),
        "uint16" => non_special!(U16Decoder),
        "uint32" => non_special!(U32Decoder),
        "uint64" => non_special!(U64Decoder::new(field)),

        // other primitives
        "bool" => non_special!(BoolDecoder),
        "float32" => non_special!(F32Decoder::new(field, ctx)?),

        // templates
        "CHandle" => non_special!(U32Decoder),
        "CStrongHandle" => non_special!(U64Decoder::new(field)),

        // pointers (?)
        "CBodyComponent" => Ok(FieldMetadata {
            special_descriptor: Some(FieldSpecialDescriptor::Pointer),
            decoder: Box::<BoolDecoder>::default(),
        }),
        "CLightComponent" => Ok(FieldMetadata {
            special_descriptor: Some(FieldSpecialDescriptor::Pointer),
            decoder: Box::<BoolDecoder>::default(),
        }),
        "CRenderComponent" => Ok(FieldMetadata {
            special_descriptor: Some(FieldSpecialDescriptor::Pointer),
            decoder: Box::<BoolDecoder>::default(),
        }),

        // other custom types
        "CUtlSymbolLarge" => non_special!(StringDecoder),
        "CUtlString" => non_special!(StringDecoder),
        // public/mathlib/vector.h
        "QAngle" => non_special!(QAngleDecoder::new(field)),
        "CNetworkedQuantizedFloat" => {
            non_special!(QuantizedFloatDecoder::new(field)?)
        }
        "GameTime_t" => non_special!(F32Decoder::new(field, ctx)?),
        "MatchID_t" => non_special!(U64Decoder::new(field)),
        // public/mathlib/vector.h
        "Vector" => non_special!(VectorDecoder::new(field, ctx)?),
        // public/mathlib/vector2d.h
        "Vector2D" => non_special!(Vector2DDecoder::new(field, ctx)?),
        // public/mathlib/vector4d.h
        "Vector4D" => non_special!(Vector4DDecoder::new(field, ctx)?),
        // game/shared/econ/econ_item_constants.h
        "itemid_t" => non_special!(U64Decoder::new(field)),
        "HeroFacetKey_t" => non_special!(U64Decoder::new(field)),
        "BloodType" => non_special!(U32Decoder),

        // exceptional specials xd
        "m_SpeechBubbles" => Ok(FieldMetadata {
            special_descriptor: Some(FieldSpecialDescriptor::DynamicSerializerArray),
            decoder: Box::<U32Decoder>::default(),
        }),
        // https://github.com/SteamDatabase/GameTracking-CS2/blob/6b3bf6ad44266e3ee4440a0b9b2fee1268812840/game/core/tools/demoinfo2/demoinfo2.txt#L155C83-L155C111
        "DOTA_CombatLogQueryProgress" => Ok(FieldMetadata {
            special_descriptor: Some(FieldSpecialDescriptor::DynamicSerializerArray),
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
    expr: Expr,
    arg: Expr,
    field: &FlattenedSerializerField,
    ctx: &FlattenedSerializerContext,
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
                decoder: Box::<U32Decoder>::default(),
            });
        }

        return visit_any(arg, field, ctx).map(|field_metadata| FieldMetadata {
            special_descriptor: Some(FieldSpecialDescriptor::DynamicArray {
                decoder: field_metadata.decoder,
            }),
            decoder: Box::<U32Decoder>::default(),
        });
    }

    return visit_ident(ident, field, ctx);
}

#[inline]
fn visit_array(
    expr: Expr,
    len: Expr,
    field: &FlattenedSerializerField,
    ctx: &FlattenedSerializerContext,
) -> Result<FieldMetadata> {
    if let Expr::Ident(ident) = expr {
        if ident == "char" {
            return Ok(FieldMetadata {
                special_descriptor: None,
                decoder: Box::<StringDecoder>::default(),
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

    visit_any(expr, field, ctx).map(|field_metadata| FieldMetadata {
        special_descriptor: Some(FieldSpecialDescriptor::FixedArray { length }),
        decoder: field_metadata.decoder,
    })
}

#[inline]
fn visit_pointer() -> Result<FieldMetadata> {
    Ok(FieldMetadata {
        special_descriptor: Some(FieldSpecialDescriptor::Pointer),
        decoder: Box::<BoolDecoder>::default(),
    })
}

#[inline]
fn visit_any(
    expr: Expr,
    field: &FlattenedSerializerField,
    ctx: &FlattenedSerializerContext,
) -> Result<FieldMetadata> {
    match expr {
        Expr::Ident(ident) => visit_ident(ident, field, ctx),
        Expr::Template { expr, arg } => visit_template(*expr, *arg, field, ctx),
        Expr::Array { expr, len } => visit_array(*expr, *len, field, ctx),
        Expr::Pointer(_) => visit_pointer(),
        _ => unreachable!(),
    }
}

pub fn get_field_metadata(
    expr: Expr,
    field: &FlattenedSerializerField,
    ctx: &FlattenedSerializerContext,
) -> Result<FieldMetadata> {
    visit_any(expr, field, ctx)
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
