use crate::{
    fielddecoder::{
        decode_bool, decode_f32, decode_i32, decode_i64, decode_qangle, decode_quantized_float,
        decode_string, decode_u32, decode_u64, decode_vec2, decode_vec3, decode_vec4, FieldDecoder,
    },
    flattenedserializers::FlattenedSerializerField,
    fnv1a::hash,
};
use std::alloc::Allocator;

// NOTE: Clone is derived because FlattenedSerializerField needs to be clonable.
#[derive(Clone, Debug)]
pub enum FieldSpecialType {
    Array { length: usize },
    VariableLengthArray,
    VariableLengthSerializerArray { element_serializer_name_hash: u64 },
}

#[derive(Clone)]
pub struct FieldMetadata<A: Allocator + Clone> {
    pub special_type: Option<FieldSpecialType>,
    pub decoder: FieldDecoder<A>,
}

const MAX_ABILITY_DRAFT_ABILITIES: usize = 48;

pub fn get_field_metadata<A: Allocator + Clone>(
    field: &FlattenedSerializerField<A>,
) -> Option<FieldMetadata<A>> {
    // TODO: create macros to make this prettier kekw
    match field.var_type_hash {
        v if v == hash(b"AbilityBarType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"AbilityID_t") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"AbilityID_t[15]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 15 }),
            decoder: decode_u32,
        }),
        v if v == hash(b"AbilityID_t[30]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 30 }),
            decoder: decode_u32,
        }),
        v if v == hash(b"AbilityID_t[5]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 5 }),
            decoder: decode_u32,
        }),
        v if v == hash(b"AbilityID_t[9]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 9 }),
            decoder: decode_u32,
        }),
        v if v == hash(b"AnimLoopMode_t") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"AttachmentHandle_t") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"AttachmentHandle_t[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: decode_u32,
        }),
        v if v == hash(b"BeamClipStyle_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"BeamType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"CBodyComponent") => {
            // does not end with a *, but apparently a pointer
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_bool,
            })
        }
        v if v == hash(b"CDOTAGameManager*") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_bool,
        }),
        v if v == hash(b"CDOTAGameRules*") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_bool,
        }),
        v if v == hash(b"CDOTASpectatorGraphManager*") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_bool,
        }),
        v if v == hash(b"CDOTA_AbilityDraftAbilityState[MAX_ABILITY_DRAFT_ABILITIES]") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::Array {
                    length: MAX_ABILITY_DRAFT_ABILITIES,
                }),
                decoder: decode_u32,
            })
        }
        v if v == hash(b"CDOTA_ArcanaDataEntity_DrowRanger*") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_bool,
        }),
        v if v == hash(b"CDOTA_ArcanaDataEntity_FacelessVoid*") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_bool,
        }),
        v if v == hash(b"CDOTA_ArcanaDataEntity_Razor*") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_bool,
        }),
        v if v == hash(b"CEntityIdentity*") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_bool,
        }),
        v if v == hash(b"CEntityIndex") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"CGameSceneNodeHandle") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"CHandle< CBaseAnimatingActivity >") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"CHandle< CBaseEntity >") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"CHandle< CBaseEntity >[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: decode_u32,
        }),
        v if v == hash(b"CHandle< CBaseEntity >[15]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 15 }),
            decoder: decode_u32,
        }),
        v if v == hash(b"CHandle< CBaseEntity >[19]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 19 }),
            decoder: decode_u32,
        }),
        v if v == hash(b"CHandle< CBaseEntity >[2]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 2 }),
            decoder: decode_u32,
        }),
        v if v == hash(b"CHandle< CBaseEntity >[35]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 35 }),
            decoder: decode_u32,
        }),
        v if v == hash(b"CHandle< CBaseEntity >[64]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 64 }),
            decoder: decode_u32,
        }),
        v if v == hash(b"CHandle< CBasePlayerController >") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"CHandle< CBasePlayerPawn >") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"CHandle< CColorCorrection >") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"CHandle< CDOTAPlayerController >") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"CHandle< CDOTASpecGraphPlayerData >[24]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 24 }),
            decoder: decode_u32,
        }),
        v if v == hash(b"CHandle< CDOTA_Ability_Meepo_DividedWeStand >") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"CHandle< CDOTA_BaseNPC >") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"CHandle< CDOTA_BaseNPC_Hero >") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"CHandle< CDOTA_Item >") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"CHandle< CDOTA_NeutralSpawner >") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"CHandle< CDotaSubquestBase >[8]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 8 }),
            decoder: decode_u32,
        }),
        v if v == hash(b"CHandle< CFogController >") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"CHandle< CTonemapController2 >") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"CLightComponent") => {
            // does not end with a *, but apparently a pointer
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_bool,
            })
        }
        v if v == hash(b"CNetworkUtlVectorBase< AbilityID_t >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthArray),
            decoder: decode_u32,
        }),
        v if v == hash(b"CNetworkUtlVectorBase< CHandle< CBaseEntity > >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthArray),
            decoder: decode_u32,
        }),
        v if v == hash(b"CNetworkUtlVectorBase< CHandle< CBaseFlex > >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthArray),
            decoder: decode_u32,
        }),
        v if v == hash(b"CNetworkUtlVectorBase< CHandle< CBaseModelEntity > >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthArray),
                decoder: decode_u32,
            })
        }
        v if v == hash(b"CNetworkUtlVectorBase< CHandle< CBasePlayerController > >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthArray),
                decoder: decode_u32,
            })
        }
        v if v == hash(b"CNetworkUtlVectorBase< CHandle< CBasePlayerPawn > >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthArray),
                decoder: decode_u32,
            })
        }
        v if v == hash(b"CNetworkUtlVectorBase< CHandle< CEconWearable > >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthArray),
                decoder: decode_u32,
            })
        }
        v if v == hash(b"CNetworkUtlVectorBase< CHandle< CIngameEvent_Base > >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthArray),
                decoder: decode_u32,
            })
        }
        v if v == hash(b"CNetworkUtlVectorBase< CHandle< CPostProcessingVolume > >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthArray),
                decoder: decode_u32,
            })
        }
        v if v == hash(b"CNetworkUtlVectorBase< CTransform >") => {
            // public/mathlib/transform.h
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthArray),
                decoder: decode_u32,
            })
        }
        v if v == hash(b"CNetworkUtlVectorBase< CUtlSymbolLarge >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthArray),
            decoder: decode_string,
        }),
        v if v == hash(b"CNetworkUtlVectorBase< NeutralSpawnBoxes_t >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                element_serializer_name_hash: hash(b"NeutralSpawnBoxes_t"),
            }),
            decoder: decode_u32,
        }),
        v if v == hash(b"CNetworkUtlVectorBase< PlayerID_t >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthArray),
            decoder: decode_u32,
        }),
        v if v == hash(b"CNetworkUtlVectorBase< QAngle >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthArray),
            decoder: decode_qangle,
        }),
        v if v == hash(b"CNetworkUtlVectorBase< RegionTriggerBoxes_t >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                element_serializer_name_hash: hash(b"RegionTriggerBoxes_t"),
            }),
            decoder: decode_u32,
        }),
        v if v == hash(b"CNetworkUtlVectorBase< Vector >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthArray),
            decoder: decode_vec3,
        }),
        v if v == hash(b"CNetworkUtlVectorBase< bool >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthArray),
            decoder: decode_bool,
        }),
        v if v == hash(b"CNetworkUtlVectorBase< float32 >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthArray),
            decoder: decode_f32,
        }),
        v if v == hash(b"CNetworkUtlVectorBase< int32 >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthArray),
            decoder: decode_i32,
        }),
        v if v == hash(b"CNetworkUtlVectorBase< uint32 >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthArray),
            decoder: decode_u32,
        }),
        v if v == hash(b"CNetworkUtlVectorBase< uint8 >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthArray),
            decoder: decode_u32,
        }),
        v if v == hash(b"CNetworkedQuantizedFloat") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_quantized_float,
        }),
        v if v == hash(b"CPlayerSlot") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"CPlayer_CameraServices*") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_bool,
        }),
        v if v == hash(b"CPlayer_MovementServices*") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_bool,
        }),
        v if v == hash(b"CRenderComponent") => {
            // does not end with a *, but apparently a pointer
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_bool,
            })
        }
        v if v == hash(b"CStrongHandle< InfoForResourceTypeCModel >") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u64,
        }),
        v if v == hash(b"CStrongHandle< InfoForResourceTypeCPostProcessingResource >") => {
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u64,
            })
        }
        v if v == hash(b"CStrongHandle< InfoForResourceTypeCTextureBase >") => {
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u64,
            })
        }
        v if v == hash(b"CStrongHandle< InfoForResourceTypeIMaterial2 >") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u64,
        }),
        v if v == hash(b"CStrongHandle< InfoForResourceTypeIParticleSystemDefinition >") => {
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u64,
            })
        }
        v if v == hash(b"CUtlString") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_string,
        }),
        v if v == hash(b"CUtlStringToken") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"CUtlSymbolLarge") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_string,
        }),
        v if v == hash(b"CUtlSymbolLarge[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: decode_string,
        }),
        v if v == hash(b"CUtlVector< CEconItemAttribute >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                element_serializer_name_hash: hash(b"CEconItemAttribute"),
            }),
            decoder: decode_u32,
        }),
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< CAnimationLayer >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                element_serializer_name_hash: hash(b"CAnimationLayer"),
            }),
            decoder: decode_u32,
        }),
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< CDOTACustomShopInfo >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: hash(b"CDOTACustomShopInfo"),
                }),
                decoder: decode_u32,
            })
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< CDOTACustomShopItemInfo >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: hash(b"CDOTACustomShopItemInfo"),
                }),
                decoder: decode_u32,
            })
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< CDOTASubChallengeInfo >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: hash(b"CDOTASubChallengeInfo"),
                }),
                decoder: decode_u32,
            })
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< CDOTA_ItemStockInfo >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: hash(b"CDOTA_ItemStockInfo"),
                }),
                decoder: decode_u32,
            })
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< CDOTA_PlayerChallengeInfo >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: hash(b"CDOTA_PlayerChallengeInfo"),
                }),
                decoder: decode_u32,
            })
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< CHeroStatueLiked >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: hash(b"CHeroStatueLiked"),
                }),
                decoder: decode_u32,
            })
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< CHeroesPerPlayer >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: hash(b"CHeroesPerPlayer"),
                }),
                decoder: decode_u32,
            })
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< DOTAThreatLevelInfo_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: hash(b"DOTAThreatLevelInfo_t"),
                }),
                decoder: decode_u32,
            })
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< DataTeamPlayer_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: hash(b"DataTeamPlayer_t"),
                }),
                decoder: decode_u32,
            })
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< EntityRenderAttribute_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: hash(b"EntityRenderAttribute_t"),
                }),
                decoder: decode_u32,
            })
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< FowBlocker_t >") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                element_serializer_name_hash: hash(b"FowBlocker_t"),
            }),
            decoder: decode_u32,
        }),
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< InGamePredictionData_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: hash(b"InGamePredictionData_t"),
                }),
                decoder: decode_u32,
            })
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< PingConfirmationState_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: hash(b"PingConfirmationState_t"),
                }),
                decoder: decode_u32,
            })
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< PlayerResourceBroadcasterData_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: hash(b"PlayerResourceBroadcasterData_t"),
                }),
                decoder: decode_u32,
            })
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< PlayerResourcePlayerData_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: hash(b"PlayerResourcePlayerData_t"),
                }),
                decoder: decode_u32,
            })
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< PlayerResourcePlayerEventData_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: hash(b"PlayerResourcePlayerEventData_t"),
                }),
                decoder: decode_u32,
            })
        }
        v if v
            == hash(
                b"CUtlVectorEmbeddedNetworkVar< PlayerResourcePlayerPeriodicResourceData_t >",
            ) =>
        {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: hash(
                        b"PlayerResourcePlayerPeriodicResourceData_t",
                    ),
                }),
                decoder: decode_u32,
            })
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< PlayerResourcePlayerTeamData_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: hash(b"PlayerResourcePlayerTeamData_t"),
                }),
                decoder: decode_u32,
            })
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< TempViewerInfo_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: hash(b"TempViewerInfo_t"),
                }),
                decoder: decode_u32,
            })
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< TierNeutralInfo_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: hash(b"TierNeutralInfo_t"),
                }),
                decoder: decode_u32,
            })
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< TreeModelReplacement_t >") => {
            Some(FieldMetadata {
                special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                    element_serializer_name_hash: hash(b"TreeModelReplacement_t"),
                }),
                decoder: decode_u32,
            })
        }
        v if v == hash(b"CavernCrawlMapVariant_t") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"Color") => {
            // public/color.h
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"CourierState_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"DOTACustomHeroPickRulesPhase_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"DOTATeam_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"DOTA_CombatLogQueryProgress") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"DOTA_HeroPickState") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"DOTA_PlayerDraftState") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"DOTA_SHOP_TYPE") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"DamageOptions_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"ECrowdLevel") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"ERoshanSpawnPhase") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"EntityDisolveType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"FowBlockerShape_t") => {
            // num
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"GameTick_t") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"GameTime_t") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_f32,
        }),
        v if v == hash(b"GameTime_t[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: decode_f32,
        }),
        v if v == hash(b"GameTime_t[15]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 15 }),
            decoder: decode_f32,
        }),
        v if v == hash(b"GameTime_t[24]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 24 }),
            decoder: decode_f32,
        }),
        v if v == hash(b"GuildID_t") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"HSequence") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"LeagueID_t") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"MatchID_t") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u64,
        }),
        v if v == hash(b"MoveCollide_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"MoveType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"PeriodicResourceID_t") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"PhysicsRagdollPose_t*") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_bool,
        }),
        v if v == hash(b"PingConfirmationIconType") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"PlayerConnectedState") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"PlayerID_t") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"PlayerID_t[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: decode_u32,
        }),
        v if v == hash(b"PlayerID_t[15]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 15 }),
            decoder: decode_u32,
        }),
        v if v == hash(b"PlayerID_t[2]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 2 }),
            decoder: decode_u32,
        }),
        v if v == hash(b"PointWorldTextJustifyHorizontal_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"PointWorldTextJustifyVertical_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"PointWorldTextReorientMode_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"QAngle") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_qangle,
        }),
        v if v == hash(b"RenderFx_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"RenderMode_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"ScoutState_t") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"ShopItemViewMode_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"SolidType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"SurroundingBoundsType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"TakeDamageFlags_t") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"ValueRemapperHapticsType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"ValueRemapperInputType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"ValueRemapperMomentumType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"ValueRemapperOutputType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"ValueRemapperRatchetType_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"Vector") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_vec3,
        }),
        v if v == hash(b"Vector2D") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_vec2,
        }),
        v if v == hash(b"Vector2D[2]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 2 }),
            decoder: decode_vec2,
        }),
        // from ability draft
        v if v == hash(b"Vector2D[100]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 100 }),
            decoder: decode_vec2,
        }),
        v if v == hash(b"Vector4D") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_vec4,
        }),
        v if v == hash(b"Vector[4]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 4 }),
            decoder: decode_vec3,
        }),
        v if v == hash(b"Vector[8]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 8 }),
            decoder: decode_vec3,
        }),
        v if v == hash(b"WeaponState_t") => {
            // enum
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"WeightedAbilitySuggestion_t[15]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 15 }),
            decoder: decode_u32,
        }),
        v if v == hash(b"WeightedAbilitySuggestion_t[3]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 3 }),
            decoder: decode_u32,
        }),
        v if v == hash(b"WeightedAbilitySuggestion_t[5]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 5 }),
            decoder: decode_u32,
        }),
        // from ability draft
        v if v == hash(b"WeightedAbilitySuggestion_t[25]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 25 }),
            decoder: decode_u32,
        }),
        v if v == hash(b"WorldGroupId_t") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"attrib_definition_index_t") => {
            // game/shared/econ/econ_item_constants.h
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"attributeprovidertypes_t") => {
            // game/shared/econ/attribute_manager.h
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"bool") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_bool,
        }),
        v if v == hash(b"bool[15]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 15 }),
            decoder: decode_bool,
        }),
        v if v == hash(b"bool[24]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 24 }),
            decoder: decode_bool,
        }),
        v if v == hash(b"bool[256]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 256 }),
            decoder: decode_bool,
        }),
        v if v == hash(b"bool[4]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 4 }),
            decoder: decode_bool,
        }),
        v if v == hash(b"bool[5]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 5 }),
            decoder: decode_bool,
        }),
        v if v == hash(b"bool[9]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 9 }),
            decoder: decode_bool,
        }),
        v if v == hash(b"char[128]") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_string,
        }),
        v if v == hash(b"char[129]") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_string,
        }),
        v if v == hash(b"char[256]") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_string,
        }),
        v if v == hash(b"char[32]") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_string,
        }),
        v if v == hash(b"char[33]") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_string,
        }),
        v if v == hash(b"char[512]") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_string,
        }),
        v if v == hash(b"char[64]") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_string,
        }),
        v if v == hash(b"float32") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_f32,
        }),
        v if v == hash(b"float32[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: decode_f32,
        }),
        v if v == hash(b"float32[15]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 15 }),
            decoder: decode_f32,
        }),
        v if v == hash(b"float32[20]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 20 }),
            decoder: decode_f32,
        }),
        v if v == hash(b"float32[24]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 24 }),
            decoder: decode_f32,
        }),
        v if v == hash(b"float32[2]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 2 }),
            decoder: decode_f32,
        }),
        v if v == hash(b"float32[3]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 3 }),
            decoder: decode_f32,
        }),
        v if v == hash(b"float32[5]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 5 }),
            decoder: decode_f32,
        }),
        // from ability draft
        v if v == hash(b"float32[100]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 100 }),
            decoder: decode_f32,
        }),
        v if v == hash(b"int16") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_i32,
        }),
        v if v == hash(b"int32") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_i32,
        }),
        v if v == hash(b"int32[100]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 100 }),
            decoder: decode_i32,
        }),
        v if v == hash(b"int32[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: decode_i32,
        }),
        v if v == hash(b"int32[13]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 13 }),
            decoder: decode_i32,
        }),
        v if v == hash(b"int32[15]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 15 }),
            decoder: decode_i32,
        }),
        v if v == hash(b"int32[24]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 24 }),
            decoder: decode_i32,
        }),
        v if v == hash(b"int32[2]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 2 }),
            decoder: decode_i32,
        }),
        v if v == hash(b"int32[3]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 3 }),
            decoder: decode_i32,
        }),
        v if v == hash(b"int32[4]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 4 }),
            decoder: decode_i32,
        }),
        v if v == hash(b"int32[5]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 5 }),
            decoder: decode_i32,
        }),
        v if v == hash(b"int32[64]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 64 }),
            decoder: decode_i32,
        }),
        v if v == hash(b"int64") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_i64,
        }),
        v if v == hash(b"int8") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_i32,
        }),
        v if v == hash(b"int8[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: decode_i32,
        }),
        v if v == hash(b"int8[24]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 24 }),
            decoder: decode_i32,
        }),
        v if v == hash(b"item_definition_index_t") => {
            // game/shared/econ/econ_item_constants.h
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"item_definition_index_t[15]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 15 }),
            decoder: decode_u32,
        }),
        v if v == hash(b"itemid_t") => {
            // game/shared/econ/econ_item_constants.h
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u64,
            })
        }
        v if v == hash(b"itemid_t[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: decode_u32,
        }),
        v if v == hash(b"m_SpeechBubbles") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::VariableLengthSerializerArray {
                element_serializer_name_hash: hash(b"CSpeechBubbleInfo"),
            }),
            decoder: decode_u32,
        }),
        v if v == hash(b"style_index_t") => {
            // game/shared/econ/econ_item_constants.h
            Some(FieldMetadata {
                special_type: None,
                decoder: decode_u32,
            })
        }
        v if v == hash(b"uint16") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"uint32") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"uint32[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: decode_u32,
        }),
        v if v == hash(b"uint32[1]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 1 }),
            decoder: decode_u32,
        }),
        v if v == hash(b"uint64") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u64,
        }),
        v if v == hash(b"uint64[256]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 256 }),
            decoder: decode_u64,
        }),
        v if v == hash(b"uint64[3]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 3 }),
            decoder: decode_u64,
        }),
        v if v == hash(b"uint64[4]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 4 }),
            decoder: decode_u64,
        }),
        v if v == hash(b"uint8") => Some(FieldMetadata {
            special_type: None,
            decoder: decode_u32,
        }),
        v if v == hash(b"uint8[10]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 10 }),
            decoder: decode_u32,
        }),
        v if v == hash(b"uint8[18]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 18 }),
            decoder: decode_u32,
        }),
        v if v == hash(b"uint8[20]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 20 }),
            decoder: decode_u32,
        }),
        v if v == hash(b"uint8[2]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 2 }),
            decoder: decode_u32,
        }),
        v if v == hash(b"uint8[4]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 4 }),
            decoder: decode_u32,
        }),
        v if v == hash(b"uint8[6]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 6 }),
            decoder: decode_u32,
        }),
        v if v == hash(b"uint8[8]") => Some(FieldMetadata {
            special_type: Some(FieldSpecialType::Array { length: 8 }),
            decoder: decode_u32,
        }),
        _ => None,
    }
}
