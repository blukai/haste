use std::io::Read;

use dungers::varint;

// NOTE: protobuf libs don't seem to be capable of no-copy, replay is extremely saturated with
// CDemoPacket messages.
//
// this exists for the sole purpose of no-copy decoding simple protobuf messages like:
//
// message CDemoSendTables {
// 	optional bytes data = 1;
// }
//
// message CDemoPacket {
// 	optional bytes data = 3;
// }

// TODO: see if the change can be made in prost that would allow to decode bytes without copying.

const TAG_TYPE_BITS: u64 = 3;
const TAG_TYPE_MASK: u64 = (1u64 << TAG_TYPE_BITS as usize) - 1;

/// this is a wrapper around [`dungers::varint::read_uvarint64`] that only maps error to
/// [`prost::DecodeError`] and does absolutely nothing else.
/// see https://sourcegraph.com/github.com/tokio-rs/prost@7968f906d2d1cf8193183873fecb025d18437cd8/-/blob/prost/src/encoding/varint.rs?L38
#[inline]
pub(crate) fn read_uvarint64<R: Read>(rdr: &mut R) -> Result<u64, prost::DecodeError> {
    let (value, _) =
        varint::read_uvarint64(rdr).map_err(|_| prost::DecodeError::new("invalid varint"))?;
    Ok(value)
}

// https://protobuf.dev/programming-guides/encoding/#structure
pub(crate) enum WireType {
    LengthDelimited = 2,
}

impl TryFrom<u64> for WireType {
    type Error = prost::DecodeError;

    #[inline]
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            2 => Ok(WireType::LengthDelimited),
            // https://sourcegraph.com/github.com/tokio-rs/prost@7968f906d2d1cf8193183873fecb025d18437cd8/-/blob/prost/src/encoding/wire_type.rs?L30
            _ => Err(prost::DecodeError::new(format!(
                "invalid wire type value: {}",
                value
            ))),
        }
    }
}

pub(crate) struct Tag {
    pub wire_type: WireType,
    pub field_number: u64,
}

pub(crate) fn read_tag<R: Read>(rdr: &mut R) -> Result<Tag, prost::DecodeError> {
    let key = read_uvarint64(rdr)?;
    let wire_type = WireType::try_from(key & TAG_TYPE_MASK)?;
    let field_number = key >> TAG_TYPE_BITS;
    Ok(Tag {
        wire_type,
        field_number,
    })
}
