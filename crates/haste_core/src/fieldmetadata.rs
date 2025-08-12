use crate::fielddecoder::{
    BoolDecoder, F32Decoder, FieldDecode, FieldDecoderConstructionError, I64Decoder,
    InvalidDecoder, QAngleDecoder, StringDecoder, U64Decoder, Vector2Decoder, Vector3Decoder,
    Vector4Decoder,
};
use crate::flattenedserializers::FlattenedSerializerField;
use crate::vartype::{self, Expr, Lit};

#[derive(thiserror::Error, Debug)]
pub enum FieldMetadataError {
    #[error(transparent)]
    VarTypeParseError(#[from] vartype::Error),
    #[error(transparent)]
    FieldDecoderConstructionError(#[from] FieldDecoderConstructionError),
    #[error("unknown array length ident: {0}")]
    UnknownArrayLengthIdent(String),
}

// NOTE: Clone is derived because FlattenedSerializerField needs to be clonable.
#[derive(Debug, Clone)]
pub(crate) enum FieldSpecialDescriptor {
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
pub(crate) struct FieldMetadata {
    pub(crate) special_descriptor: Option<FieldSpecialDescriptor>,
    pub(crate) decoder: Box<dyn FieldDecode>,
}

impl Default for FieldMetadata {
    #[inline]
    fn default() -> Self {
        Self {
            special_descriptor: None,
            decoder: Box::<InvalidDecoder>::default(),
        }
    }
}

#[inline]
fn visit_ident(
    ident: &str,
    field: &FlattenedSerializerField,
) -> Result<FieldMetadata, FieldMetadataError> {
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

    macro_rules! pointer {
        () => {
            Ok(FieldMetadata {
                special_descriptor: Some(FieldSpecialDescriptor::Pointer),
                decoder: Box::<BoolDecoder>::default(),
            })
        };
    }

    match ident {
        // primitives
        "int8" => non_special!(I64Decoder),
        "int16" => non_special!(I64Decoder),
        "int32" => non_special!(I64Decoder),
        "int64" => non_special!(I64Decoder),
        "bool" => non_special!(BoolDecoder),
        "float32" => non_special!(F32Decoder::new(field)?),

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
        "CUtlSymbolLarge" => non_special!(StringDecoder),
        "CUtlString" => non_special!(StringDecoder),
        // public/mathlib/vector.h
        "QAngle" => non_special!(QAngleDecoder::new(field)),
        // NOTE: not all quantized floats are actually quantized (if bit_count is 0 or 32 it's
        // not!) F32Decoder will determine which kind of f32 decoder to use.
        "CNetworkedQuantizedFloat" => non_special!(F32Decoder::new(field)?),
        "GameTime_t" => non_special!(F32Decoder::new(field)?),
        // public/mathlib/vector.h
        "Vector" => non_special!(Vector3Decoder::new(field)?),
        // public/mathlib/vector2d.h
        "Vector2D" => non_special!(Vector2Decoder::new(field)?),
        // public/mathlib/vector4d.h
        "Vector4D" => non_special!(Vector4Decoder::new(field)?),

        // exceptional specials xd
        "m_SpeechBubbles" => Ok(FieldMetadata {
            special_descriptor: Some(FieldSpecialDescriptor::DynamicSerializerArray),
            decoder: Box::<U64Decoder>::default(),
        }),
        // https://github.com/SteamDatabase/GameTracking-CS2/blob/6b3bf6ad44266e3ee4440a0b9b2fee1268812840/game/core/tools/demoinfo2/demoinfo2.txt#L155C83-L155C111
        "DOTA_CombatLogQueryProgress" => Ok(FieldMetadata {
            special_descriptor: Some(FieldSpecialDescriptor::DynamicSerializerArray),
            decoder: Box::<U64Decoder>::default(),
        }),

        // default
        _ => Ok(FieldMetadata {
            special_descriptor: None,
            decoder: Box::new(U64Decoder::new(field)),
        }),
    }
}

#[inline]
fn visit_template(
    expr: Expr,
    arg: Expr,
    field: &FlattenedSerializerField,
) -> Result<FieldMetadata, FieldMetadataError> {
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
                decoder: Box::<U64Decoder>::default(),
            });
        }

        return visit_any(arg, field).map(|field_metadata| FieldMetadata {
            special_descriptor: Some(FieldSpecialDescriptor::DynamicArray {
                decoder: field_metadata.decoder,
            }),
            decoder: Box::<U64Decoder>::default(),
        });
    }

    visit_ident(ident, field)
}

#[inline]
fn visit_array(
    expr: Expr,
    len: Expr,
    field: &FlattenedSerializerField,
) -> Result<FieldMetadata, FieldMetadataError> {
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
            "DOTA_ABILITY_DRAFT_HEROES_PER_GAME" => Ok(10),
            _ => Err(FieldMetadataError::UnknownArrayLengthIdent(
                ident.to_owned(),
            )),
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
fn visit_pointer() -> Result<FieldMetadata, FieldMetadataError> {
    Ok(FieldMetadata {
        special_descriptor: Some(FieldSpecialDescriptor::Pointer),
        decoder: Box::<BoolDecoder>::default(),
    })
}

#[inline]
fn visit_any(
    expr: Expr,
    field: &FlattenedSerializerField,
) -> Result<FieldMetadata, FieldMetadataError> {
    match expr {
        Expr::Ident(ident) => visit_ident(ident, field),
        Expr::Template { expr, arg } => visit_template(*expr, *arg, field),
        Expr::Array { expr, len } => visit_array(*expr, *len, field),
        Expr::Pointer(_) => visit_pointer(),
        _ => unreachable!(),
    }
}

pub(crate) fn get_field_metadata(
    field: &FlattenedSerializerField,
    var_type: &String,
) -> Result<FieldMetadata, FieldMetadataError> {
    let expr = vartype::parse(var_type.as_str())?;
    visit_any(expr, field)
}
