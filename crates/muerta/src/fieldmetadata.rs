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
        v if v == fnv1a::hash_bytes(b"AbilityBarType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"AbilityID_t") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"AbilityID_t[15]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 15 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"AbilityID_t[30]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 30 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"AbilityID_t[5]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 5 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"AbilityID_t[9]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 9 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"AnimLoopMode_t") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"AttachmentHandle_t") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"AttachmentHandle_t[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"BeamClipStyle_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"BeamType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CBodyComponent") => {
            // does not end with a *, but apparently a pointer
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<BoolDecoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CDOTAGameManager*") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CDOTAGameRules*") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CDOTASpectatorGraphManager*") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v
            == fnv1a::hash_bytes(
                b"CDOTA_AbilityDraftAbilityState[MAX_ABILITY_DRAFT_ABILITIES]",
            ) =>
        {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::Array {
                    length: MAX_ABILITY_DRAFT_ABILITIES,
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CDOTA_ArcanaDataEntity_DrowRanger*") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CDOTA_ArcanaDataEntity_FacelessVoid*") => {
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<BoolDecoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CDOTA_ArcanaDataEntity_Razor*") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CEntityIdentity*") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CEntityIndex") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CGameSceneNodeHandle") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CHandle< CBaseAnimatingActivity >") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CHandle< CBaseEntity >") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CHandle< CBaseEntity >[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CHandle< CBaseEntity >[15]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 15 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CHandle< CBaseEntity >[19]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 19 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CHandle< CBaseEntity >[2]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 2 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CHandle< CBaseEntity >[35]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 35 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CHandle< CBaseEntity >[64]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 64 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CHandle< CBasePlayerController >") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CHandle< CBasePlayerPawn >") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CHandle< CColorCorrection >") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CHandle< CDOTAPlayerController >") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CHandle< CDOTASpecGraphPlayerData >[24]") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::Array { length: 24 }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CHandle< CDOTA_Ability_Meepo_DividedWeStand >") => {
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CHandle< CDOTA_BaseNPC >") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CHandle< CDOTA_BaseNPC_Hero >") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CHandle< CDOTA_Item >") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CHandle< CDOTA_NeutralSpawner >") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CHandle< CDotaSubquestBase >[8]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 8 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CHandle< CFogController >") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CHandle< CTonemapController2 >") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CLightComponent") => {
            // does not end with a *, but apparently a pointer
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<BoolDecoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CNetworkUtlVectorBase< AbilityID_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthArray),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CNetworkUtlVectorBase< CHandle< CBaseEntity > >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthArray),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CNetworkUtlVectorBase< CHandle< CBaseFlex > >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthArray),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CNetworkUtlVectorBase< CHandle< CBaseModelEntity > >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthArray),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v
            == fnv1a::hash_bytes(b"CNetworkUtlVectorBase< CHandle< CBasePlayerController > >") =>
        {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthArray),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CNetworkUtlVectorBase< CHandle< CBasePlayerPawn > >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthArray),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CNetworkUtlVectorBase< CHandle< CEconWearable > >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthArray),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CNetworkUtlVectorBase< CHandle< CIngameEvent_Base > >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthArray),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v
            == fnv1a::hash_bytes(b"CNetworkUtlVectorBase< CHandle< CPostProcessingVolume > >") =>
        {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthArray),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CNetworkUtlVectorBase< CTransform >") => {
            // public/mathlib/transform.h
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthArray),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CNetworkUtlVectorBase< CUtlSymbolLarge >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthArray),
                decoder: Box::<StringDecoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CNetworkUtlVectorBase< NeutralSpawnBoxes_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_bytes(b"NeutralSpawnBoxes_t"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CNetworkUtlVectorBase< PlayerID_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthArray),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CNetworkUtlVectorBase< QAngle >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthArray),
            decoder: Box::new(QAngleDecoder::new(field)),
        }),
        v if v == fnv1a::hash_bytes(b"CNetworkUtlVectorBase< RegionTriggerBoxes_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_bytes(b"RegionTriggerBoxes_t"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CNetworkUtlVectorBase< Vector >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthArray),
            decoder: Box::new(Vec3Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_bytes(b"CNetworkUtlVectorBase< bool >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthArray),
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CNetworkUtlVectorBase< float32 >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthArray),
            decoder: Box::new(F32Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_bytes(b"CNetworkUtlVectorBase< int32 >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthArray),
            decoder: Box::<I32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CNetworkUtlVectorBase< uint32 >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthArray),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CNetworkUtlVectorBase< uint8 >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthArray),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CNetworkedQuantizedFloat") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::new(QuantizedFloatDecoder::new(field)?),
        }),
        v if v == fnv1a::hash_bytes(b"CPlayerSlot") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CPlayer_CameraServices*") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CPlayer_MovementServices*") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CRenderComponent") => {
            // does not end with a *, but apparently a pointer
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<BoolDecoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CStrongHandle< InfoForResourceTypeCModel >") => {
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::new(U64Decoder::new(field)),
            })
        }
        v if v
            == fnv1a::hash_bytes(
                b"CStrongHandle< InfoForResourceTypeCPostProcessingResource >",
            ) =>
        {
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::new(U64Decoder::new(field)),
            })
        }
        v if v == fnv1a::hash_bytes(b"CStrongHandle< InfoForResourceTypeCTextureBase >") => {
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::new(U64Decoder::new(field)),
            })
        }
        v if v == fnv1a::hash_bytes(b"CStrongHandle< InfoForResourceTypeIMaterial2 >") => {
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::new(U64Decoder::new(field)),
            })
        }
        v if v
            == fnv1a::hash_bytes(
                b"CStrongHandle< InfoForResourceTypeIParticleSystemDefinition >",
            ) =>
        {
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::new(U64Decoder::new(field)),
            })
        }
        v if v == fnv1a::hash_bytes(b"CUtlString") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<StringDecoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CUtlStringToken") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CUtlSymbolLarge") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<StringDecoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CUtlSymbolLarge[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: Box::<StringDecoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CUtlVector< CEconItemAttribute >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                element_serializer_name_hash: fnv1a::hash_bytes(b"CEconItemAttribute"),
            }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"CUtlVectorEmbeddedNetworkVar< CAnimationLayer >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_bytes(b"CAnimationLayer"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CUtlVectorEmbeddedNetworkVar< CDOTACustomShopInfo >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_bytes(b"CDOTACustomShopInfo"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CUtlVectorEmbeddedNetworkVar< CDOTACustomShopItemInfo >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_bytes(b"CDOTACustomShopItemInfo"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CUtlVectorEmbeddedNetworkVar< CDOTASubChallengeInfo >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_bytes(b"CDOTASubChallengeInfo"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CUtlVectorEmbeddedNetworkVar< CDOTA_ItemStockInfo >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_bytes(b"CDOTA_ItemStockInfo"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v
            == fnv1a::hash_bytes(b"CUtlVectorEmbeddedNetworkVar< CDOTA_PlayerChallengeInfo >") =>
        {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_bytes(b"CDOTA_PlayerChallengeInfo"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CUtlVectorEmbeddedNetworkVar< CHeroStatueLiked >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_bytes(b"CHeroStatueLiked"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CUtlVectorEmbeddedNetworkVar< CHeroesPerPlayer >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_bytes(b"CHeroesPerPlayer"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CUtlVectorEmbeddedNetworkVar< DOTAThreatLevelInfo_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_bytes(b"DOTAThreatLevelInfo_t"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CUtlVectorEmbeddedNetworkVar< DataTeamPlayer_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_bytes(b"DataTeamPlayer_t"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CUtlVectorEmbeddedNetworkVar< EntityRenderAttribute_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_bytes(b"EntityRenderAttribute_t"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CUtlVectorEmbeddedNetworkVar< FowBlocker_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_bytes(b"FowBlocker_t"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CUtlVectorEmbeddedNetworkVar< InGamePredictionData_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_bytes(b"InGamePredictionData_t"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CUtlVectorEmbeddedNetworkVar< PingConfirmationState_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_bytes(b"PingConfirmationState_t"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v
            == fnv1a::hash_bytes(
                b"CUtlVectorEmbeddedNetworkVar< PlayerResourceBroadcasterData_t >",
            ) =>
        {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_bytes(
                        b"PlayerResourceBroadcasterData_t",
                    ),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v
            == fnv1a::hash_bytes(b"CUtlVectorEmbeddedNetworkVar< PlayerResourcePlayerData_t >") =>
        {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_bytes(b"PlayerResourcePlayerData_t"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v
            == fnv1a::hash_bytes(
                b"CUtlVectorEmbeddedNetworkVar< PlayerResourcePlayerEventData_t >",
            ) =>
        {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_bytes(
                        b"PlayerResourcePlayerEventData_t",
                    ),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v
            == fnv1a::hash_bytes(
                b"CUtlVectorEmbeddedNetworkVar< PlayerResourcePlayerPeriodicResourceData_t >",
            ) =>
        {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_bytes(
                        b"PlayerResourcePlayerPeriodicResourceData_t",
                    ),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v
            == fnv1a::hash_bytes(
                b"CUtlVectorEmbeddedNetworkVar< PlayerResourcePlayerTeamData_t >",
            ) =>
        {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_bytes(
                        b"PlayerResourcePlayerTeamData_t",
                    ),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CUtlVectorEmbeddedNetworkVar< TempViewerInfo_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_bytes(b"TempViewerInfo_t"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CUtlVectorEmbeddedNetworkVar< TierNeutralInfo_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_bytes(b"TierNeutralInfo_t"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CUtlVectorEmbeddedNetworkVar< TreeModelReplacement_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: fnv1a::hash_bytes(b"TreeModelReplacement_t"),
                }),
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CavernCrawlMapVariant_t") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"Color") => {
            // public/color.h
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"CourierState_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"DOTACustomHeroPickRulesPhase_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"DOTATeam_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"DOTA_CombatLogQueryProgress") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"DOTA_HeroPickState") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"DOTA_PlayerDraftState") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"DOTA_SHOP_TYPE") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"DamageOptions_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"ECrowdLevel") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"ERoshanSpawnPhase") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"EntityDisolveType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"FowBlockerShape_t") => {
            // num
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"GameTick_t") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"GameTime_t") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::new(F32Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_bytes(b"GameTime_t[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: Box::new(F32Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_bytes(b"GameTime_t[15]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 15 }),
            decoder: Box::new(F32Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_bytes(b"GameTime_t[24]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 24 }),
            decoder: Box::new(F32Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_bytes(b"GuildID_t") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"HSequence") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"LeagueID_t") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"MatchID_t") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::new(U64Decoder::new(field)),
        }),
        v if v == fnv1a::hash_bytes(b"MoveCollide_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"MoveType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"PeriodicResourceID_t") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"PhysicsRagdollPose_t*") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"PingConfirmationIconType") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"PlayerConnectedState") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"PlayerID_t") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"PlayerID_t[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"PlayerID_t[15]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 15 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"PlayerID_t[2]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 2 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"PointWorldTextJustifyHorizontal_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"PointWorldTextJustifyVertical_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"PointWorldTextReorientMode_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"QAngle") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::new(QAngleDecoder::new(field)),
        }),
        v if v == fnv1a::hash_bytes(b"RenderFx_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"RenderMode_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"ScoutState_t") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"ShopItemViewMode_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"SolidType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"SurroundingBoundsType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"TakeDamageFlags_t") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"ValueRemapperHapticsType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"ValueRemapperInputType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"ValueRemapperMomentumType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"ValueRemapperOutputType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"ValueRemapperRatchetType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"Vector") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::new(Vec3Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_bytes(b"Vector2D") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::new(Vec2Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_bytes(b"Vector2D[2]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 2 }),
            decoder: Box::new(Vec2Decoder::new(field)?),
        }),
        // from ability draft
        v if v == fnv1a::hash_bytes(b"Vector2D[100]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 100 }),
            decoder: Box::new(Vec2Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_bytes(b"Vector4D") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::new(Vec4Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_bytes(b"Vector[4]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 4 }),
            decoder: Box::new(Vec3Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_bytes(b"Vector[8]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 8 }),
            decoder: Box::new(Vec3Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_bytes(b"WeaponState_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"WeightedAbilitySuggestion_t[15]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 15 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"WeightedAbilitySuggestion_t[3]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 3 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"WeightedAbilitySuggestion_t[5]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 5 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        // from ability draft
        v if v == fnv1a::hash_bytes(b"WeightedAbilitySuggestion_t[25]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 25 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"WorldGroupId_t") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"attrib_definition_index_t") => {
            // game/shared/econ/econ_item_constants.h
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"attributeprovidertypes_t") => {
            // game/shared/econ/attribute_manager.h
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"bool") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"bool[15]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 15 }),
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"bool[24]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 24 }),
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"bool[256]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 256 }),
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"bool[4]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 4 }),
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"bool[5]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 5 }),
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"bool[9]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 9 }),
            decoder: Box::<BoolDecoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"char[128]") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<StringDecoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"char[129]") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<StringDecoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"char[256]") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<StringDecoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"char[32]") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<StringDecoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"char[33]") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<StringDecoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"char[512]") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<StringDecoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"char[64]") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<StringDecoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"float32") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::new(F32Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_bytes(b"float32[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: Box::new(F32Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_bytes(b"float32[15]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 15 }),
            decoder: Box::new(F32Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_bytes(b"float32[20]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 20 }),
            decoder: Box::new(F32Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_bytes(b"float32[24]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 24 }),
            decoder: Box::new(F32Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_bytes(b"float32[2]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 2 }),
            decoder: Box::new(F32Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_bytes(b"float32[3]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 3 }),
            decoder: Box::new(F32Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_bytes(b"float32[5]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 5 }),
            decoder: Box::new(F32Decoder::new(field)?),
        }),
        // from ability draft
        v if v == fnv1a::hash_bytes(b"float32[100]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 100 }),
            decoder: Box::new(F32Decoder::new(field)?),
        }),
        // from 7364789105
        v if v == fnv1a::hash_bytes(b"float32[4]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 4 }),
            decoder: Box::new(F32Decoder::new(field)?),
        }),
        v if v == fnv1a::hash_bytes(b"int16") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<I32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"int32") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<I32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"int32[100]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 100 }),
            decoder: Box::<I32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"int32[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: Box::<I32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"int32[13]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 13 }),
            decoder: Box::<I32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"int32[15]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 15 }),
            decoder: Box::<I32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"int32[24]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 24 }),
            decoder: Box::<I32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"int32[2]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 2 }),
            decoder: Box::<I32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"int32[3]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 3 }),
            decoder: Box::<I32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"int32[4]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 4 }),
            decoder: Box::<I32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"int32[5]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 5 }),
            decoder: Box::<I32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"int32[64]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 64 }),
            decoder: Box::<I32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"int64") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<I64Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"int8") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<I32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"int8[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: Box::<I32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"int8[24]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 24 }),
            decoder: Box::<I32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"item_definition_index_t") => {
            // game/shared/econ/econ_item_constants.h
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"item_definition_index_t[15]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 15 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"itemid_t") => {
            // game/shared/econ/econ_item_constants.h
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::new(U64Decoder::new(field)),
            })
        }
        v if v == fnv1a::hash_bytes(b"itemid_t[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"m_SpeechBubbles") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                element_serializer_name_hash: fnv1a::hash_bytes(b"CSpeechBubbleInfo"),
            }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"style_index_t") => {
            // game/shared/econ/econ_item_constants.h
            Some(FieldMetadata {
                special_type: None,
                decoder: Box::<U32Decoder>::default(),
            })
        }
        v if v == fnv1a::hash_bytes(b"uint16") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"uint32") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"uint32[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"uint32[1]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 1 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"uint64") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::new(U64Decoder::new(field)),
        }),
        v if v == fnv1a::hash_bytes(b"uint64[256]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 256 }),
            decoder: Box::new(U64Decoder::new(field)),
        }),
        v if v == fnv1a::hash_bytes(b"uint64[3]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 3 }),
            decoder: Box::new(U64Decoder::new(field)),
        }),
        v if v == fnv1a::hash_bytes(b"uint64[4]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 4 }),
            decoder: Box::new(U64Decoder::new(field)),
        }),
        v if v == fnv1a::hash_bytes(b"uint8") => Some(FieldMetadata {
            special_type: None,
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"uint8[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"uint8[18]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 18 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"uint8[20]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 20 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"uint8[2]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 2 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"uint8[4]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 4 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"uint8[6]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 6 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        v if v == fnv1a::hash_bytes(b"uint8[8]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 8 }),
            decoder: Box::<U32Decoder>::default(),
        }),
        _ => None,
    };
    Ok(field_metadata)
}
