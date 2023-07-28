use crate::{
    bitbuf::{self, BitReader},
    fieldkind::FieldKind,
    fieldvalue::FieldValue,
    flattenedserializers::FlattenedSerializerField,
    fnv1a::hash,
    quantizedfloat::{self, QuantizedFloat},
};
use std::alloc::Allocator;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // crate
    #[error(transparent)]
    BitBuf(#[from] bitbuf::Error),
    #[error(transparent)]
    QuantizedFloat(#[from] quantizedfloat::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

const TICK_INTERVAL: f32 = 1.0 / 30.0;
const MAX_ABILITY_DRAFT_ABILITIES: usize = 48;

pub type FieldDecoder<A> =
    fn(field: &FlattenedSerializerField<A>, br: &mut BitReader, alloc: A) -> Result<FieldValue<A>>;

fn decode_u32<A: Allocator + Clone>(
    _field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
    _alloc: A,
) -> Result<FieldValue<A>> {
    Ok(br.read_uvarint32()?.into())
}

#[inline(always)]
fn internal_decode_u64_fixed64<A: Allocator + Clone>(
    _field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
) -> Result<u64> {
    let mut bytes = [0u8; 8];
    br.read_bytes(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

fn decode_u64<A: Allocator + Clone>(
    field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
    _alloc: A,
) -> Result<FieldValue<A>> {
    if field.is_var_encoder_hash_eq(hash(b"fixed64")) {
        return Ok(internal_decode_u64_fixed64(field, br)?.into());
    }

    Ok(br.read_uvarint64()?.into())
}

fn decode_i32<A: Allocator + Clone>(
    _field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
    _alloc: A,
) -> Result<FieldValue<A>> {
    Ok(br.read_varint32()?.into())
}

fn decode_i64<A: Allocator + Clone>(
    _field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
    _alloc: A,
) -> Result<FieldValue<A>> {
    Ok(br.read_varint64()?.into())
}

#[inline(always)]
fn internal_decode_quantized_float<A: Allocator + Clone>(
    field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
) -> Result<f32> {
    let qf = QuantizedFloat::new(
        field.bit_count.unwrap_or_default(),
        field.encode_flags.unwrap_or_default(),
        field.low_value.unwrap_or_default(),
        field.high_value.unwrap_or_default(),
    )?;
    Ok(qf.decode(br)?)
}

fn decode_quantized_float<A: Allocator + Clone>(
    field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
    _alloc: A,
) -> Result<FieldValue<A>> {
    Ok(internal_decode_quantized_float(field, br)?.into())
}

#[inline(always)]
fn internal_decode_f32_simulation_time<A: Allocator + Clone>(
    _field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
) -> Result<f32> {
    Ok(br
        .read_uvarint32()
        .map(|value| value as f32 * TICK_INTERVAL)?)
}

#[inline(always)]
fn internal_decode_f32_coord<A: Allocator + Clone>(
    _field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
) -> Result<f32> {
    Ok(br.read_bitcoord()?)
}

#[inline(always)]
fn internal_decode_f32_noscale(br: &mut BitReader) -> Result<f32> {
    Ok(br.read_bitfloat()?)
}

#[inline(always)]
fn internal_decode_f32<A: Allocator + Clone>(
    field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
) -> Result<f32> {
    if field.var_name_hash == hash(b"m_flSimulationTime")
        || field.var_name_hash == hash(b"m_flAnimTime")
    {
        return internal_decode_f32_simulation_time(field, br);
    }

    if field.is_var_encoder_hash_eq(hash(b"coord")) {
        return internal_decode_f32_coord(field, br);
    }

    let bit_count = field.bit_count.unwrap_or_default();
    // why would it be greater than 32? :thinking:
    if bit_count == 0 || bit_count >= 32 {
        return internal_decode_f32_noscale(br);
    }

    internal_decode_quantized_float(field, br)
}

fn decode_f32<A: Allocator + Clone>(
    field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
    _alloc: A,
) -> Result<FieldValue<A>> {
    Ok(internal_decode_f32(field, br)?.into())
}

fn decode_bool<A: Allocator + Clone>(
    _field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
    _alloc: A,
) -> Result<FieldValue<A>> {
    Ok(br.read_bool()?.into())
}

#[inline(always)]
fn internal_decode_qangle_pitch_yaw<A: Allocator + Clone>(
    field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
    alloc: A,
) -> Result<Box<[f32; 3], A>> {
    let mut vec3 = Box::new_in([0.0f32; 3], alloc);
    let bit_count = field.bit_count.unwrap_or_default() as usize;
    vec3[0] = br.read_bitangle(bit_count)?;
    vec3[1] = br.read_bitangle(bit_count)?;
    Ok(vec3)
}

#[inline(always)]
fn internal_decode_qangle_no_bit_count<A: Allocator + Clone>(
    _field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
    alloc: A,
) -> Result<Box<[f32; 3], A>> {
    let mut vec3 = Box::new_in([0.0f32; 3], alloc);

    let rx = br.read_bool()?;
    let ry = br.read_bool()?;
    let rz = br.read_bool()?;

    if rx {
        vec3[0] = br.read_bitcoord()?;
    }
    if ry {
        vec3[1] = br.read_bitcoord()?;
    }
    if rz {
        vec3[2] = br.read_bitcoord()?;
    }

    Ok(vec3)
}

fn decode_qangle<A: Allocator + Clone>(
    field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
    alloc: A,
) -> Result<FieldValue<A>> {
    let bit_count = field.bit_count.unwrap_or_default();

    if field.is_var_encoder_hash_eq(hash(b"qangle_pitch_yaw")) {
        return Ok(internal_decode_qangle_pitch_yaw(field, br, alloc)?.into());
    }

    if bit_count == 0 {
        return Ok(internal_decode_qangle_no_bit_count(field, br, alloc)?.into());
    }

    unimplemented!("other qangle decoder")
}

fn decode_vec3<A: Allocator + Clone>(
    field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
    alloc: A,
) -> Result<FieldValue<A>> {
    let mut vec3 = Box::new_in([0.0f32; 3], alloc);
    vec3[0] = internal_decode_f32(field, br)?;
    vec3[1] = internal_decode_f32(field, br)?;
    vec3[2] = internal_decode_f32(field, br)?;
    Ok(vec3.into())
}

fn decode_vec2<A: Allocator + Clone>(
    field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
    alloc: A,
) -> Result<FieldValue<A>> {
    let mut vec2 = Box::new_in([0.0f32; 2], alloc);
    vec2[0] = internal_decode_f32(field, br)?;
    vec2[1] = internal_decode_f32(field, br)?;
    Ok(vec2.into())
}

fn decode_vec4<A: Allocator + Clone>(
    field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
    alloc: A,
) -> Result<FieldValue<A>> {
    let mut vec4 = Box::new_in([0.0f32; 4], alloc);
    vec4[0] = internal_decode_f32(field, br)?;
    vec4[1] = internal_decode_f32(field, br)?;
    vec4[2] = internal_decode_f32(field, br)?;
    vec4[3] = internal_decode_f32(field, br)?;
    Ok(vec4.into())
}

fn decode_string<A: Allocator + Clone>(
    _field: &FlattenedSerializerField<A>,
    br: &mut BitReader,
    alloc: A,
) -> Result<FieldValue<A>> {
    let mut buf = [0u8; 1024];
    let num_chars = br.read_string(&mut buf, false)?;
    Ok(buf[..num_chars].to_vec_in(alloc).into())
}

pub(crate) fn supplement<A: Allocator + Clone>(field: &mut FlattenedSerializerField<A>) {
    // TODO: create macros to make this prettier kekw

    match field.var_type_hash {
        v if v == hash(b"AbilityBarType_t") => {
            // enum
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"AbilityID_t") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"AbilityID_t[15]") => {
            field.kind = Some(FieldKind::FixedArray { size: 15 });
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"AbilityID_t[30]") => {
            field.kind = Some(FieldKind::FixedArray { size: 30 });
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"AbilityID_t[5]") => {
            field.kind = Some(FieldKind::FixedArray { size: 5 });
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"AbilityID_t[9]") => {
            field.kind = Some(FieldKind::FixedArray { size: 9 });
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"AnimLoopMode_t") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"AttachmentHandle_t") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"AttachmentHandle_t[10]") => {
            field.kind = Some(FieldKind::FixedArray { size: 10 });
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"BeamClipStyle_t") => {
            // enum
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"BeamType_t") => {
            // enum
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CBodyComponent") => {
            // does not end with a *, but apparently a pointer
            field.decoder = Some(decode_bool);
        }
        v if v == hash(b"CDOTAGameManager*") => {
            field.decoder = Some(decode_bool);
        }
        v if v == hash(b"CDOTAGameRules*") => {
            field.decoder = Some(decode_bool);
        }
        v if v == hash(b"CDOTASpectatorGraphManager*") => {
            field.decoder = Some(decode_bool);
        }
        v if v == hash(b"CDOTA_AbilityDraftAbilityState[MAX_ABILITY_DRAFT_ABILITIES]") => {
            // TODO: try to parse ability draft match -> shouldn't this be a table?
            field.kind = Some(FieldKind::FixedArray {
                size: MAX_ABILITY_DRAFT_ABILITIES,
            });
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CDOTA_ArcanaDataEntity_DrowRanger*") => {
            field.decoder = Some(decode_bool);
        }
        v if v == hash(b"CDOTA_ArcanaDataEntity_FacelessVoid*") => {
            field.decoder = Some(decode_bool);
        }
        v if v == hash(b"CDOTA_ArcanaDataEntity_Razor*") => {
            field.decoder = Some(decode_bool);
        }
        v if v == hash(b"CEntityIdentity*") => {
            field.decoder = Some(decode_bool);
        }
        v if v == hash(b"CEntityIndex") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CGameSceneNodeHandle") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CHandle< CBaseAnimatingActivity >") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CHandle< CBaseEntity >") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CHandle< CBaseEntity >[10]") => {
            field.kind = Some(FieldKind::FixedArray { size: 10 });
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CHandle< CBaseEntity >[15]") => {
            field.kind = Some(FieldKind::FixedArray { size: 15 });
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CHandle< CBaseEntity >[19]") => {
            field.kind = Some(FieldKind::FixedArray { size: 19 });
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CHandle< CBaseEntity >[2]") => {
            field.kind = Some(FieldKind::FixedArray { size: 2 });
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CHandle< CBaseEntity >[35]") => {
            field.kind = Some(FieldKind::FixedArray { size: 35 });
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CHandle< CBaseEntity >[64]") => {
            field.kind = Some(FieldKind::FixedArray { size: 64 });
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CHandle< CBasePlayerController >") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CHandle< CBasePlayerPawn >") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CHandle< CColorCorrection >") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CHandle< CDOTAPlayerController >") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CHandle< CDOTASpecGraphPlayerData >[24]") => {
            field.kind = Some(FieldKind::FixedArray { size: 24 });
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CHandle< CDOTA_Ability_Meepo_DividedWeStand >") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CHandle< CDOTA_BaseNPC >") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CHandle< CDOTA_BaseNPC_Hero >") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CHandle< CDOTA_Item >") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CHandle< CDOTA_NeutralSpawner >") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CHandle< CDotaSubquestBase >[8]") => {
            field.kind = Some(FieldKind::FixedArray { size: 8 });
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CHandle< CFogController >") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CHandle< CTonemapController2 >") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CLightComponent") => {
            // does not end with a *, but apparently a pointer
            field.decoder = Some(decode_bool);
        }
        v if v == hash(b"CNetworkUtlVectorBase< AbilityID_t >") => {
            field.kind = Some(FieldKind::DynamicArray);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CNetworkUtlVectorBase< CHandle< CBaseEntity > >") => {
            field.kind = Some(FieldKind::DynamicArray);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CNetworkUtlVectorBase< CHandle< CBaseFlex > >") => {
            field.kind = Some(FieldKind::DynamicArray);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CNetworkUtlVectorBase< CHandle< CBaseModelEntity > >") => {
            field.kind = Some(FieldKind::DynamicArray);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CNetworkUtlVectorBase< CHandle< CBasePlayerController > >") => {
            field.kind = Some(FieldKind::DynamicArray);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CNetworkUtlVectorBase< CHandle< CBasePlayerPawn > >") => {
            field.kind = Some(FieldKind::DynamicArray);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CNetworkUtlVectorBase< CHandle< CEconWearable > >") => {
            field.kind = Some(FieldKind::DynamicArray);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CNetworkUtlVectorBase< CHandle< CIngameEvent_Base > >") => {
            field.kind = Some(FieldKind::DynamicArray);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CNetworkUtlVectorBase< CHandle< CPostProcessingVolume > >") => {
            field.kind = Some(FieldKind::DynamicArray);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CNetworkUtlVectorBase< CTransform >") => {
            // public/mathlib/transform.h
            field.kind = Some(FieldKind::DynamicTable);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CNetworkUtlVectorBase< CUtlSymbolLarge >") => {
            field.kind = Some(FieldKind::DynamicArray);
            field.decoder = Some(decode_string);
        }
        v if v == hash(b"CNetworkUtlVectorBase< NeutralSpawnBoxes_t >") => {
            field.kind = Some(FieldKind::DynamicTable);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CNetworkUtlVectorBase< PlayerID_t >") => {
            field.kind = Some(FieldKind::DynamicArray);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CNetworkUtlVectorBase< QAngle >") => {
            field.kind = Some(FieldKind::DynamicArray);
            field.decoder = Some(decode_qangle);
        }
        v if v == hash(b"CNetworkUtlVectorBase< RegionTriggerBoxes_t >") => {
            field.kind = Some(FieldKind::DynamicTable);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CNetworkUtlVectorBase< Vector >") => {
            field.kind = Some(FieldKind::DynamicArray);
            field.decoder = Some(decode_vec3);
        }
        v if v == hash(b"CNetworkUtlVectorBase< bool >") => {
            field.kind = Some(FieldKind::DynamicArray);
            field.decoder = Some(decode_bool);
        }
        v if v == hash(b"CNetworkUtlVectorBase< float32 >") => {
            field.kind = Some(FieldKind::DynamicArray);
            field.decoder = Some(decode_f32);
        }
        v if v == hash(b"CNetworkUtlVectorBase< int32 >") => {
            field.kind = Some(FieldKind::DynamicArray);
            field.decoder = Some(decode_i32);
        }
        v if v == hash(b"CNetworkUtlVectorBase< uint32 >") => {
            field.kind = Some(FieldKind::DynamicArray);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CNetworkUtlVectorBase< uint8 >") => {
            field.kind = Some(FieldKind::DynamicArray);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CNetworkedQuantizedFloat") => {
            field.decoder = Some(decode_quantized_float);
        }
        v if v == hash(b"CPlayerSlot") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CPlayer_CameraServices*") => {
            field.decoder = Some(decode_bool);
        }
        v if v == hash(b"CPlayer_MovementServices*") => {
            field.decoder = Some(decode_bool);
        }
        v if v == hash(b"CRenderComponent") => {
            // does not end with a *, but apparently a pointer
            field.decoder = Some(decode_bool);
        }
        v if v == hash(b"CStrongHandle< InfoForResourceTypeCModel >") => {
            field.decoder = Some(decode_u64);
        }
        v if v == hash(b"CStrongHandle< InfoForResourceTypeCPostProcessingResource >") => {
            field.decoder = Some(decode_u64);
        }
        v if v == hash(b"CStrongHandle< InfoForResourceTypeCTextureBase >") => {
            field.decoder = Some(decode_u64);
        }
        v if v == hash(b"CStrongHandle< InfoForResourceTypeIMaterial2 >") => {
            field.decoder = Some(decode_u64);
        }
        v if v == hash(b"CStrongHandle< InfoForResourceTypeIParticleSystemDefinition >") => {
            field.decoder = Some(decode_u64);
        }
        v if v == hash(b"CUtlString") => {
            field.decoder = Some(decode_string);
        }
        v if v == hash(b"CUtlStringToken") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CUtlSymbolLarge") => {
            field.decoder = Some(decode_string);
        }
        v if v == hash(b"CUtlSymbolLarge[10]") => {
            field.kind = Some(FieldKind::FixedArray { size: 10 });
            field.decoder = Some(decode_string);
        }
        v if v == hash(b"CUtlVector< CEconItemAttribute >") => {
            field.kind = Some(FieldKind::DynamicTable);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< CAnimationLayer >") => {
            field.kind = Some(FieldKind::DynamicTable);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< CDOTACustomShopInfo >") => {
            field.kind = Some(FieldKind::DynamicTable);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< CDOTACustomShopItemInfo >") => {
            field.kind = Some(FieldKind::DynamicTable);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< CDOTASubChallengeInfo >") => {
            field.kind = Some(FieldKind::DynamicTable);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< CDOTA_ItemStockInfo >") => {
            field.kind = Some(FieldKind::DynamicTable);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< CDOTA_PlayerChallengeInfo >") => {
            field.kind = Some(FieldKind::DynamicTable);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< CHeroStatueLiked >") => {
            field.kind = Some(FieldKind::DynamicTable);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< CHeroesPerPlayer >") => {
            field.kind = Some(FieldKind::DynamicTable);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< DOTAThreatLevelInfo_t >") => {
            field.kind = Some(FieldKind::DynamicTable);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< DataTeamPlayer_t >") => {
            field.kind = Some(FieldKind::DynamicTable);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< EntityRenderAttribute_t >") => {
            field.kind = Some(FieldKind::DynamicTable);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< FowBlocker_t >") => {
            field.kind = Some(FieldKind::DynamicTable);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< InGamePredictionData_t >") => {
            field.kind = Some(FieldKind::DynamicTable);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< PingConfirmationState_t >") => {
            field.kind = Some(FieldKind::DynamicTable);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< PlayerResourceBroadcasterData_t >") => {
            field.kind = Some(FieldKind::DynamicTable);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< PlayerResourcePlayerData_t >") => {
            field.kind = Some(FieldKind::DynamicTable);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< PlayerResourcePlayerEventData_t >") => {
            field.kind = Some(FieldKind::DynamicTable);
            field.decoder = Some(decode_u32);
        }
        v if v
            == hash(
                b"CUtlVectorEmbeddedNetworkVar< PlayerResourcePlayerPeriodicResourceData_t >",
            ) =>
        {
            field.kind = Some(FieldKind::DynamicTable);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< PlayerResourcePlayerTeamData_t >") => {
            field.kind = Some(FieldKind::DynamicTable);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< TempViewerInfo_t >") => {
            field.kind = Some(FieldKind::DynamicTable);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< TierNeutralInfo_t >") => {
            field.kind = Some(FieldKind::DynamicTable);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CUtlVectorEmbeddedNetworkVar< TreeModelReplacement_t >") => {
            field.kind = Some(FieldKind::DynamicTable);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CavernCrawlMapVariant_t") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"Color") => {
            // public/color.h
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"CourierState_t") => {
            // enum
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"DOTACustomHeroPickRulesPhase_t") => {
            // enum
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"DOTATeam_t") => {
            // enum
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"DOTA_CombatLogQueryProgress") => {
            field.kind = Some(FieldKind::DynamicTable);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"DOTA_HeroPickState") => {
            // enum
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"DOTA_PlayerDraftState") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"DOTA_SHOP_TYPE") => {
            // enum
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"DamageOptions_t") => {
            // enum
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"ECrowdLevel") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"ERoshanSpawnPhase") => {
            // enum
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"EntityDisolveType_t") => {
            // enum
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"FowBlockerShape_t") => {
            // num
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"GameTick_t") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"GameTime_t") => {
            field.decoder = Some(decode_f32);
        }
        v if v == hash(b"GameTime_t[10]") => {
            field.kind = Some(FieldKind::FixedArray { size: 10 });
            field.decoder = Some(decode_f32);
        }
        v if v == hash(b"GameTime_t[15]") => {
            field.kind = Some(FieldKind::FixedArray { size: 15 });
            field.decoder = Some(decode_f32);
        }
        v if v == hash(b"GameTime_t[24]") => {
            field.kind = Some(FieldKind::FixedArray { size: 24 });
            field.decoder = Some(decode_f32);
        }
        v if v == hash(b"GuildID_t") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"HSequence") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"LeagueID_t") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"MatchID_t") => {
            field.decoder = Some(decode_u64);
        }
        v if v == hash(b"MoveCollide_t") => {
            // enum
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"MoveType_t") => {
            // enum
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"PeriodicResourceID_t") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"PhysicsRagdollPose_t*") => {
            field.decoder = Some(decode_bool);
        }
        v if v == hash(b"PingConfirmationIconType") => {
            // enum
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"PlayerConnectedState") => {
            // enum
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"PlayerID_t") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"PlayerID_t[10]") => {
            field.kind = Some(FieldKind::FixedArray { size: 10 });
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"PlayerID_t[15]") => {
            field.kind = Some(FieldKind::FixedArray { size: 15 });
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"PlayerID_t[2]") => {
            field.kind = Some(FieldKind::FixedArray { size: 2 });
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"PointWorldTextJustifyHorizontal_t") => {
            // enum
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"PointWorldTextJustifyVertical_t") => {
            // enum
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"PointWorldTextReorientMode_t") => {
            // enum
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"QAngle") => {
            field.decoder = Some(decode_qangle);
        }
        v if v == hash(b"RenderFx_t") => {
            // enum
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"RenderMode_t") => {
            // enum
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"ScoutState_t") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"ShopItemViewMode_t") => {
            // enum
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"SolidType_t") => {
            // enum
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"SurroundingBoundsType_t") => {
            // enum
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"TakeDamageFlags_t") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"ValueRemapperHapticsType_t") => {
            // enum
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"ValueRemapperInputType_t") => {
            // enum
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"ValueRemapperMomentumType_t") => {
            // enum
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"ValueRemapperOutputType_t") => {
            // enum
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"ValueRemapperRatchetType_t") => {
            // enum
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"Vector") => {
            field.decoder = Some(decode_vec3);
        }
        v if v == hash(b"Vector2D") => {
            field.decoder = Some(decode_vec2);
        }
        v if v == hash(b"Vector2D[2]") => {
            field.kind = Some(FieldKind::FixedArray { size: 2 });
            field.decoder = Some(decode_vec2);
        }
        v if v == hash(b"Vector4D") => {
            field.decoder = Some(decode_vec4);
        }
        v if v == hash(b"Vector[4]") => {
            field.kind = Some(FieldKind::FixedArray { size: 4 });
            field.decoder = Some(decode_vec3);
        }
        v if v == hash(b"Vector[8]") => {
            field.kind = Some(FieldKind::FixedArray { size: 8 });
            field.decoder = Some(decode_vec3);
        }
        v if v == hash(b"WeaponState_t") => {
            // enum
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"WeightedAbilitySuggestion_t[15]") => {
            field.kind = Some(FieldKind::FixedTable { size: 15 });
        }
        v if v == hash(b"WeightedAbilitySuggestion_t[3]") => {
            field.kind = Some(FieldKind::FixedTable { size: 3 });
        }
        v if v == hash(b"WeightedAbilitySuggestion_t[5]") => {
            field.kind = Some(FieldKind::FixedTable { size: 5 });
        }
        v if v == hash(b"WorldGroupId_t") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"attrib_definition_index_t") => {
            // game/shared/econ/econ_item_constants.h
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"attributeprovidertypes_t") => {
            // game/shared/econ/attribute_manager.h
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"bool") => {
            field.decoder = Some(decode_bool);
        }
        v if v == hash(b"bool[15]") => {
            field.kind = Some(FieldKind::FixedArray { size: 15 });
            field.decoder = Some(decode_bool);
        }
        v if v == hash(b"bool[24]") => {
            field.kind = Some(FieldKind::FixedArray { size: 24 });
            field.decoder = Some(decode_bool);
        }
        v if v == hash(b"bool[256]") => {
            field.kind = Some(FieldKind::FixedArray { size: 256 });
            field.decoder = Some(decode_bool);
        }
        v if v == hash(b"bool[4]") => {
            field.kind = Some(FieldKind::FixedArray { size: 4 });
            field.decoder = Some(decode_bool);
        }
        v if v == hash(b"bool[5]") => {
            field.kind = Some(FieldKind::FixedArray { size: 5 });
            field.decoder = Some(decode_bool);
        }
        v if v == hash(b"bool[9]") => {
            field.kind = Some(FieldKind::FixedArray { size: 9 });
            field.decoder = Some(decode_bool);
        }
        v if v == hash(b"char[128]") => {
            field.decoder = Some(decode_string);
        }
        v if v == hash(b"char[129]") => {
            field.decoder = Some(decode_string);
        }
        v if v == hash(b"char[256]") => {
            field.decoder = Some(decode_string);
        }
        v if v == hash(b"char[32]") => {
            field.decoder = Some(decode_string);
        }
        v if v == hash(b"char[33]") => {
            field.decoder = Some(decode_string);
        }
        v if v == hash(b"char[512]") => {
            field.decoder = Some(decode_string);
        }
        v if v == hash(b"char[64]") => {
            field.decoder = Some(decode_string);
        }
        v if v == hash(b"float32") => {
            field.decoder = Some(decode_f32);
        }
        v if v == hash(b"float32[10]") => {
            field.kind = Some(FieldKind::FixedArray { size: 10 });
            field.decoder = Some(decode_f32);
        }
        v if v == hash(b"float32[15]") => {
            field.kind = Some(FieldKind::FixedArray { size: 15 });
            field.decoder = Some(decode_f32);
        }
        v if v == hash(b"float32[20]") => {
            field.kind = Some(FieldKind::FixedArray { size: 20 });
            field.decoder = Some(decode_f32);
        }
        v if v == hash(b"float32[24]") => {
            field.kind = Some(FieldKind::FixedArray { size: 24 });
            field.decoder = Some(decode_f32);
        }
        v if v == hash(b"float32[2]") => {
            field.kind = Some(FieldKind::FixedArray { size: 2 });
            field.decoder = Some(decode_f32);
        }
        v if v == hash(b"float32[3]") => {
            field.kind = Some(FieldKind::FixedArray { size: 3 });
            field.decoder = Some(decode_f32);
        }
        v if v == hash(b"float32[5]") => {
            field.kind = Some(FieldKind::FixedArray { size: 5 });
            field.decoder = Some(decode_f32);
        }
        v if v == hash(b"int16") => {
            field.decoder = Some(decode_i32);
        }
        v if v == hash(b"int32") => {
            field.decoder = Some(decode_i32);
        }
        v if v == hash(b"int32[100]") => {
            field.kind = Some(FieldKind::FixedArray { size: 100 });
            field.decoder = Some(decode_i32);
        }
        v if v == hash(b"int32[10]") => {
            field.kind = Some(FieldKind::FixedArray { size: 10 });
            field.decoder = Some(decode_i32);
        }
        v if v == hash(b"int32[13]") => {
            field.kind = Some(FieldKind::FixedArray { size: 13 });
            field.decoder = Some(decode_i32);
        }
        v if v == hash(b"int32[15]") => {
            field.kind = Some(FieldKind::FixedArray { size: 15 });
            field.decoder = Some(decode_i32);
        }
        v if v == hash(b"int32[24]") => {
            field.kind = Some(FieldKind::FixedArray { size: 24 });
            field.decoder = Some(decode_i32);
        }
        v if v == hash(b"int32[2]") => {
            field.kind = Some(FieldKind::FixedArray { size: 2 });
            field.decoder = Some(decode_i32);
        }
        v if v == hash(b"int32[3]") => {
            field.kind = Some(FieldKind::FixedArray { size: 3 });
            field.decoder = Some(decode_i32);
        }
        v if v == hash(b"int32[4]") => {
            field.kind = Some(FieldKind::FixedArray { size: 4 });
            field.decoder = Some(decode_i32);
        }
        v if v == hash(b"int32[5]") => {
            field.kind = Some(FieldKind::FixedArray { size: 5 });
            field.decoder = Some(decode_i32);
        }
        v if v == hash(b"int32[64]") => {
            field.kind = Some(FieldKind::FixedArray { size: 64 });
            field.decoder = Some(decode_i32);
        }
        v if v == hash(b"int64") => {
            field.decoder = Some(decode_i64);
        }
        v if v == hash(b"int8") => {
            field.decoder = Some(decode_i32);
        }
        v if v == hash(b"int8[10]") => {
            field.kind = Some(FieldKind::FixedArray { size: 10 });
            field.decoder = Some(decode_i32);
        }
        v if v == hash(b"int8[24]") => {
            field.kind = Some(FieldKind::FixedArray { size: 24 });
            field.decoder = Some(decode_i32);
        }
        v if v == hash(b"item_definition_index_t") => {
            // game/shared/econ/econ_item_constants.h
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"item_definition_index_t[15]") => {
            field.kind = Some(FieldKind::FixedArray { size: 15 });
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"itemid_t") => {
            // game/shared/econ/econ_item_constants.h
            field.decoder = Some(decode_u64);
        }
        v if v == hash(b"itemid_t[10]") => {
            field.kind = Some(FieldKind::FixedArray { size: 10 });
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"m_SpeechBubbles") => {
            field.kind = Some(FieldKind::DynamicTable);
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"style_index_t") => {
            // game/shared/econ/econ_item_constants.h
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"uint16") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"uint32") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"uint32[10]") => {
            field.kind = Some(FieldKind::FixedArray { size: 10 });
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"uint32[1]") => {
            field.kind = Some(FieldKind::FixedArray { size: 1 });
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"uint64") => {
            field.decoder = Some(decode_u64);
        }
        v if v == hash(b"uint64[256]") => {
            field.kind = Some(FieldKind::FixedArray { size: 256 });
            field.decoder = Some(decode_u64);
        }
        v if v == hash(b"uint64[3]") => {
            field.kind = Some(FieldKind::FixedArray { size: 3 });
            field.decoder = Some(decode_u64);
        }
        v if v == hash(b"uint64[4]") => {
            field.kind = Some(FieldKind::FixedArray { size: 4 });
            field.decoder = Some(decode_u64);
        }
        v if v == hash(b"uint8") => {
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"uint8[10]") => {
            field.kind = Some(FieldKind::FixedArray { size: 10 });
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"uint8[18]") => {
            field.kind = Some(FieldKind::FixedArray { size: 18 });
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"uint8[20]") => {
            field.kind = Some(FieldKind::FixedArray { size: 20 });
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"uint8[2]") => {
            field.kind = Some(FieldKind::FixedArray { size: 2 });
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"uint8[4]") => {
            field.kind = Some(FieldKind::FixedArray { size: 4 });
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"uint8[6]") => {
            field.kind = Some(FieldKind::FixedArray { size: 6 });
            field.decoder = Some(decode_u32);
        }
        v if v == hash(b"uint8[8]") => {
            field.kind = Some(FieldKind::FixedArray { size: 8 });
            field.decoder = Some(decode_u32);
        }
        _ => {
            panic!("unhandled flattened serializer var type: {}", unsafe {
                std::str::from_utf8_unchecked(&field.var_type)
            });
        }
    }
}
