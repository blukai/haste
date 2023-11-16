use crate::{
    fielddecoder::{
        self, BoolDecoder, F32Decoder, FieldDecode, I32Decoder, I64Decoder, QAngleDecoder,
        QuantizedFloatDecoder, StringDecoder, U32Decoder, U64Decoder, Vec2Decoder, Vec3Decoder,
        Vec4Decoder,
    },
    flattenedserializers::FlattenedSerializerField,
    fnv1a,
};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // crate
    #[error(transparent)]
    FieldDecoder(#[from] fielddecoder::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

// NOTE: Clone is derived because FlattenedSerializerField needs to be clonable.
#[derive(Debug, Clone)]
pub enum FieldSpecialType {
    Array { length: usize },
    VariableLengthArray,
    VariableLengthSerializerArray { element_serializer_name_hash: u64 },
    // TODO: make use of the poiter special type (atm it's useless; but it's
    // supposed to be used to determine whether a new "entity" must be created
    // (and deserialized value of the pointer field (/bool) must not be
    // stored)).
    Pointer,
}

#[derive(Debug, Clone)]
pub struct FieldMetadata {
    pub special_type: Option<FieldSpecialType>,
    pub decoder: Box<dyn FieldDecode>,
}

const MAX_ABILITY_DRAFT_ABILITIES: usize = 48;

pub fn get_field_metadata(field: &FlattenedSerializerField) -> Result<Option<FieldMetadata>> {
    // TODO: create macros to make this prettier kekw
    let field_metadata = match field.var_type_hash {
        v if v == fnv1a::hash_u8(b"AbilityBarType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"AbilityID_t") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"AbilityID_t[15]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 15 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"AbilityID_t[30]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 30 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"AbilityID_t[5]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 5 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"AbilityID_t[9]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 9 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"AnimLoopMode_t") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"AttachmentHandle_t") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"AttachmentHandle_t[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"BeamClipStyle_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"BeamType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CBodyComponent") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Pointer),
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CDOTAGameManager*") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Pointer),
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CDOTAGameRules*") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Pointer),
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CDOTASpectatorGraphManager*") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Pointer),
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v
            == fnv1a::hash_u8(b"CDOTA_AbilityDraftAbilityState[MAX_ABILITY_DRAFT_ABILITIES]") =>
        {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::Array {
                    length: MAX_ABILITY_DRAFT_ABILITIES,
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CDOTA_ArcanaDataEntity_DrowRanger*") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Pointer),
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CDOTA_ArcanaDataEntity_FacelessVoid*") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Pointer),
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CDOTA_ArcanaDataEntity_Razor*") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Pointer),
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CEntityIdentity*") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Pointer),
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CEntityIndex") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CGameSceneNodeHandle") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CHandle< CBaseAnimatingActivity >") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CHandle< CBaseEntity >") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CHandle< CBaseEntity >[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CHandle< CBaseEntity >[15]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 15 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CHandle< CBaseEntity >[19]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 19 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CHandle< CBaseEntity >[2]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 2 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CHandle< CBaseEntity >[35]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 35 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CHandle< CBaseEntity >[64]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 64 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CHandle< CBasePlayerController >") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CHandle< CBasePlayerPawn >") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CHandle< CColorCorrection >") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CHandle< CDOTAPlayerController >") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CHandle< CDOTASpecGraphPlayerData >[24]") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::Array { length: 24 }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CHandle< CDOTA_Ability_Meepo_DividedWeStand >") => {
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CHandle< CDOTA_BaseNPC >") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CHandle< CDOTA_BaseNPC_Hero >") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CHandle< CDOTA_Item >") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CHandle< CDOTA_NeutralSpawner >") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CHandle< CDotaSubquestBase >[8]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 8 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CHandle< CFogController >") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CHandle< CTonemapController2 >") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CLightComponent") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Pointer),
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CNetworkUtlVectorBase< AbilityID_t >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthArray),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CNetworkUtlVectorBase< CHandle< CBaseEntity > >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthArray),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CNetworkUtlVectorBase< CHandle< CBaseFlex > >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthArray),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CNetworkUtlVectorBase< CHandle< CBaseModelEntity > >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthArray),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CNetworkUtlVectorBase< CHandle< CBasePlayerController > >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthArray),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CNetworkUtlVectorBase< CHandle< CBasePlayerPawn > >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthArray),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CNetworkUtlVectorBase< CHandle< CEconWearable > >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthArray),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CNetworkUtlVectorBase< CHandle< CIngameEvent_Base > >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthArray),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CNetworkUtlVectorBase< CHandle< CPostProcessingVolume > >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthArray),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CNetworkUtlVectorBase< CTransform >") => {
            // public/mathlib/transform.h
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthArray),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CNetworkUtlVectorBase< CUtlSymbolLarge >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthArray),
                decoder: Box::<StringDecoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CNetworkUtlVectorBase< NeutralSpawnBoxes_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_u8(b"NeutralSpawnBoxes_t"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CNetworkUtlVectorBase< PlayerID_t >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthArray),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CNetworkUtlVectorBase< QAngle >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthArray),
            decoder: Box::new(QAngleDecoder::new(field)),
        }),
        v if v == fnv1a::hash_u8(b"CNetworkUtlVectorBase< RegionTriggerBoxes_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_u8(b"RegionTriggerBoxes_t"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CNetworkUtlVectorBase< Vector >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthArray),
            decoder: Box::new(Vec3Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_u8(b"CNetworkUtlVectorBase< bool >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthArray),
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CNetworkUtlVectorBase< float32 >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthArray),
            decoder: Box::new(F32Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_u8(b"CNetworkUtlVectorBase< int32 >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthArray),
            decoder: Box::<I32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CNetworkUtlVectorBase< uint32 >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthArray),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CNetworkUtlVectorBase< uint8 >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthArray),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CNetworkedQuantizedFloat") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::new(QuantizedFloatDecoder::new(field)?),
        }),
        v if v == fnv1a::hash_u8(b"CPlayerSlot") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CPlayer_CameraServices*") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Pointer),
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CPlayer_MovementServices*") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Pointer),
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CRenderComponent") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Pointer),
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CStrongHandle< InfoForResourceTypeCModel >") => {
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::new(U64Decoder::new(field)),
            })
        }
        v if v
            == fnv1a::hash_u8(b"CStrongHandle< InfoForResourceTypeCPostProcessingResource >") =>
        {
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::new(U64Decoder::new(field)),
            })
        }
        v if v == fnv1a::hash_u8(b"CStrongHandle< InfoForResourceTypeCTextureBase >") => {
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::new(U64Decoder::new(field)),
            })
        }
        v if v == fnv1a::hash_u8(b"CStrongHandle< InfoForResourceTypeIMaterial2 >") => {
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::new(U64Decoder::new(field)),
            })
        }
        v if v
            == fnv1a::hash_u8(b"CStrongHandle< InfoForResourceTypeIParticleSystemDefinition >") =>
        {
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::new(U64Decoder::new(field)),
            })
        }
        v if v == fnv1a::hash_u8(b"CUtlString") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<StringDecoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CUtlStringToken") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CUtlSymbolLarge") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<StringDecoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CUtlSymbolLarge[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: Box::<StringDecoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CUtlVector< CEconItemAttribute >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                element_serializer_name_hash: fnv1a::hash_u8(b"CEconItemAttribute"),
            }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"CUtlVectorEmbeddedNetworkVar< CAnimationLayer >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_u8(b"CAnimationLayer"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CUtlVectorEmbeddedNetworkVar< CDOTACustomShopInfo >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_u8(b"CDOTACustomShopInfo"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CUtlVectorEmbeddedNetworkVar< CDOTACustomShopItemInfo >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_u8(b"CDOTACustomShopItemInfo"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CUtlVectorEmbeddedNetworkVar< CDOTASubChallengeInfo >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_u8(b"CDOTASubChallengeInfo"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CUtlVectorEmbeddedNetworkVar< CDOTA_ItemStockInfo >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_u8(b"CDOTA_ItemStockInfo"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CUtlVectorEmbeddedNetworkVar< CDOTA_PlayerChallengeInfo >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_u8(b"CDOTA_PlayerChallengeInfo"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CUtlVectorEmbeddedNetworkVar< CHeroStatueLiked >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_u8(b"CHeroStatueLiked"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CUtlVectorEmbeddedNetworkVar< CHeroesPerPlayer >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_u8(b"CHeroesPerPlayer"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CUtlVectorEmbeddedNetworkVar< DOTAThreatLevelInfo_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_u8(b"DOTAThreatLevelInfo_t"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CUtlVectorEmbeddedNetworkVar< DataTeamPlayer_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_u8(b"DataTeamPlayer_t"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CUtlVectorEmbeddedNetworkVar< EntityRenderAttribute_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_u8(b"EntityRenderAttribute_t"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CUtlVectorEmbeddedNetworkVar< FowBlocker_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_u8(b"FowBlocker_t"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CUtlVectorEmbeddedNetworkVar< InGamePredictionData_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_u8(b"InGamePredictionData_t"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CUtlVectorEmbeddedNetworkVar< PingConfirmationState_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_u8(b"PingConfirmationState_t"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v
            == fnv1a::hash_u8(
                b"CUtlVectorEmbeddedNetworkVar< PlayerResourceBroadcasterData_t >",
            ) =>
        {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_u8(
                        b"PlayerResourceBroadcasterData_t",
                    ),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CUtlVectorEmbeddedNetworkVar< PlayerResourcePlayerData_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_u8(b"PlayerResourcePlayerData_t"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v
            == fnv1a::hash_u8(
                b"CUtlVectorEmbeddedNetworkVar< PlayerResourcePlayerEventData_t >",
            ) =>
        {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_u8(
                        b"PlayerResourcePlayerEventData_t",
                    ),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v
            == fnv1a::hash_u8(
                b"CUtlVectorEmbeddedNetworkVar< PlayerResourcePlayerPeriodicResourceData_t >",
            ) =>
        {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_u8(
                        b"PlayerResourcePlayerPeriodicResourceData_t",
                    ),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v
            == fnv1a::hash_u8(
                b"CUtlVectorEmbeddedNetworkVar< PlayerResourcePlayerTeamData_t >",
            ) =>
        {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_u8(b"PlayerResourcePlayerTeamData_t"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CUtlVectorEmbeddedNetworkVar< TempViewerInfo_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_u8(b"TempViewerInfo_t"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CUtlVectorEmbeddedNetworkVar< TierNeutralInfo_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_u8(b"TierNeutralInfo_t"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CUtlVectorEmbeddedNetworkVar< TreeModelReplacement_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_u8(b"TreeModelReplacement_t"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CavernCrawlMapVariant_t") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"Color") => {
            // public/color.h
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"CourierState_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"DOTACustomHeroPickRulesPhase_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"DOTATeam_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"DOTA_CombatLogQueryProgress") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                // https://github.com/SteamDatabase/GameTracking-CS2/blob/6b3bf6ad44266e3ee4440a0b9b2fee1268812840/game/core/tools/demoinfo2/demoinfo2.txt#L155C83-L155C111
                element_serializer_name_hash: fnv1a::hash_u8(b"CDOTA_CombatLogQueryProgress"),
            }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"DOTA_HeroPickState") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"DOTA_PlayerDraftState") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"DOTA_SHOP_TYPE") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"DamageOptions_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"ECrowdLevel") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"ERoshanSpawnPhase") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"EntityDisolveType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"FowBlockerShape_t") => {
            // num
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"GameTick_t") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"GameTime_t") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::new(F32Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_u8(b"GameTime_t[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: Box::new(F32Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_u8(b"GameTime_t[15]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 15 }),
            decoder: Box::new(F32Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_u8(b"GameTime_t[24]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 24 }),
            decoder: Box::new(F32Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_u8(b"GuildID_t") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"HSequence") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"LeagueID_t") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"MatchID_t") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::new(U64Decoder::new(field)),
        }),
        v if v == fnv1a::hash_u8(b"MoveCollide_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"MoveType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"PeriodicResourceID_t") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"PhysicsRagdollPose_t*") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Pointer),
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"PingConfirmationIconType") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"PlayerConnectedState") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"PlayerID_t") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"PlayerID_t[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"PlayerID_t[15]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 15 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"PlayerID_t[2]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 2 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"PointWorldTextJustifyHorizontal_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"PointWorldTextJustifyVertical_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"PointWorldTextReorientMode_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"QAngle") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::new(QAngleDecoder::new(field)),
        }),
        v if v == fnv1a::hash_u8(b"RenderFx_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"RenderMode_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"ScoutState_t") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"ShopItemViewMode_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"SolidType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"SurroundingBoundsType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"TakeDamageFlags_t") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"ValueRemapperHapticsType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"ValueRemapperInputType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"ValueRemapperMomentumType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"ValueRemapperOutputType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"ValueRemapperRatchetType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"Vector") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::new(Vec3Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_u8(b"Vector2D") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::new(Vec2Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_u8(b"Vector2D[2]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 2 }),
            decoder: Box::new(Vec2Decoder::new(field)?),
        }),
        // from ability draft
        v if v == fnv1a::hash_u8(b"Vector2D[100]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 100 }),
            decoder: Box::new(Vec2Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_u8(b"Vector4D") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::new(Vec4Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_u8(b"Vector[4]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 4 }),
            decoder: Box::new(Vec3Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_u8(b"Vector[8]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 8 }),
            decoder: Box::new(Vec3Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_u8(b"WeaponState_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"WeightedAbilitySuggestion_t[15]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 15 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"WeightedAbilitySuggestion_t[3]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 3 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"WeightedAbilitySuggestion_t[5]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 5 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        // from ability draft
        v if v == fnv1a::hash_u8(b"WeightedAbilitySuggestion_t[25]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 25 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"WorldGroupId_t") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"attrib_definition_index_t") => {
            // game/shared/econ/econ_item_constants.h
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"attributeprovidertypes_t") => {
            // game/shared/econ/attribute_manager.h
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"bool") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"bool[15]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 15 }),
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"bool[24]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 24 }),
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"bool[256]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 256 }),
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"bool[4]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 4 }),
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"bool[5]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 5 }),
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"bool[9]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 9 }),
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"char[128]") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<StringDecoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"char[129]") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<StringDecoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"char[256]") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<StringDecoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"char[32]") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<StringDecoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"char[33]") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<StringDecoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"char[512]") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<StringDecoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"char[64]") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<StringDecoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"float32") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::new(F32Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_u8(b"float32[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: Box::new(F32Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_u8(b"float32[15]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 15 }),
            decoder: Box::new(F32Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_u8(b"float32[20]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 20 }),
            decoder: Box::new(F32Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_u8(b"float32[24]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 24 }),
            decoder: Box::new(F32Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_u8(b"float32[2]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 2 }),
            decoder: Box::new(F32Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_u8(b"float32[3]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 3 }),
            decoder: Box::new(F32Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_u8(b"float32[5]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 5 }),
            decoder: Box::new(F32Decoder::new(field)?),
        }),
        // from ability draft
        v if v == fnv1a::hash_u8(b"float32[100]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 100 }),
            decoder: Box::new(F32Decoder::new(field)?),
        }),
        // from 7364789105
        v if v == fnv1a::hash_u8(b"float32[4]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 4 }),
            decoder: Box::new(F32Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_u8(b"int16") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<I32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"int32") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<I32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"int32[100]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 100 }),
            decoder: Box::<I32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"int32[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: Box::<I32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"int32[13]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 13 }),
            decoder: Box::<I32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"int32[15]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 15 }),
            decoder: Box::<I32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"int32[24]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 24 }),
            decoder: Box::<I32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"int32[2]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 2 }),
            decoder: Box::<I32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"int32[3]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 3 }),
            decoder: Box::<I32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"int32[4]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 4 }),
            decoder: Box::<I32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"int32[5]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 5 }),
            decoder: Box::<I32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"int32[64]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 64 }),
            decoder: Box::<I32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"int64") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<I64Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"int8") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<I32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"int8[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: Box::<I32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"int8[24]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 24 }),
            decoder: Box::<I32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"item_definition_index_t") => {
            // game/shared/econ/econ_item_constants.h
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"item_definition_index_t[15]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 15 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"itemid_t") => {
            // game/shared/econ/econ_item_constants.h
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::new(U64Decoder::new(field)),
            })
        }
        v if v == fnv1a::hash_u8(b"itemid_t[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"m_SpeechBubbles") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                element_serializer_name_hash: fnv1a::hash_u8(b"CSpeechBubbleInfo"),
            }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"style_index_t") => {
            // game/shared/econ/econ_item_constants.h
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_u8(b"uint16") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"uint32") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"uint32[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"uint32[1]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 1 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"uint64") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::new(U64Decoder::new(field)),
        }),
        v if v == fnv1a::hash_u8(b"uint64[256]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 256 }),
            decoder: Box::new(U64Decoder::new(field)),
        }),
        v if v == fnv1a::hash_u8(b"uint64[3]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 3 }),
            decoder: Box::new(U64Decoder::new(field)),
        }),
        v if v == fnv1a::hash_u8(b"uint64[4]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 4 }),
            decoder: Box::new(U64Decoder::new(field)),
        }),
        v if v == fnv1a::hash_u8(b"uint8") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"uint8[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"uint8[18]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 18 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"uint8[20]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 20 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"uint8[2]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 2 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"uint8[4]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 4 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"uint8[6]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 6 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_u8(b"uint8[8]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 8 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        _ => None,
    };
    Ok(field_metadata)
}
