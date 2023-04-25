#[allow(clippy::all)]
pub mod protos {
    include!(concat!(env!("OUT_DIR"), "/_.rs"));
}

#[cfg(test)]
mod tests {
    use prost::Message;
    use std::fs::{self};

    use crate::protos;

    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    #[test]
    fn metadata() -> Result<()> {
        let bytes = fs::read("../fixtures/7124038650_1040730225.meta")?;

        let proto = protos::CdotaMatchMetadataFile::decode(&bytes[..])?;
        dbg!(&proto);

        // CMsgDOTAMatch has private_metadata_key field that might be used to decode this field.
        // src: https://github.com/ValvePython/dota2/issues/9

        // let private_metadata = proto.private_metadata.as_ref().expect("private metadata");
        // let proto = protos::CdotaMatchPrivateMetadata::decode(&private_metadata[..])?;
        // dbg!(proto);

        Ok(())
    }
}
