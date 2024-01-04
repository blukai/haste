type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

mod var_type {
    // NOTE: a build script can't depend on the crate it builds, include! is a
    // workaround stolen from https://stackoverflow.com/a/77017512

    #[allow(dead_code)]
    mod haste_dota2 {
        pub mod demofile {
            include!("../haste_dota2/src/demofile.rs");
        }
    }

    mod haste_dota2_deflat {
        pub mod var_type {
            pub mod tokenizer {
                include!("../haste_dota2_deflat/src/var_type/tokenizer.rs");
            }
        }
    }

    use haste_common::varint;
    use haste_dota2::demofile::DemoFile;
    use haste_dota2_deflat::var_type::tokenizer::{Token, Tokenizer};
    use haste_dota2_protos::{
        prost::Message, CDemoSendTables, CsvcMsgFlattenedSerializer, EDemoCommands,
    };
    use std::{
        collections::HashSet,
        fs::File,
        io::{BufReader, SeekFrom},
    };

    // NOTE: this is a super tiny demo that can be created in a localhost lobby
    // in dota.
    //
    // create a local lobby (all pick is fine) -> go through draft ->  server
    // will start recording demo automatically, look for messages that signal
    // that demo recording has started - all the content that we need here will
    // be written immediately, so it is safe to leave the game pretty much
    // immediately.
    const DEMO_FILE_PATH: &str = "../../fixtures/auto-20240102-2233-start-____.dem";

    fn get_flattened_serializer() -> super::Result<CsvcMsgFlattenedSerializer> {
        let file = File::open(DEMO_FILE_PATH)?;
        let buf_reader = BufReader::new(file);

        let mut demo_file = DemoFile::from_reader(buf_reader);
        let _demo_header = demo_file.read_demo_header()?;

        loop {
            let cmd_header = demo_file.read_cmd_header()?;
            match cmd_header.command {
                // DemSendTables cmd is sent only once
                EDemoCommands::DemSendTables => {
                    let cmd = CDemoSendTables::decode(demo_file.read_cmd(&cmd_header)?)?;
                    let msg = {
                        let mut data = &cmd.data.unwrap_or_default()[..];
                        let (_size, _count) = varint::read_uvarint32(&mut data)?;
                        CsvcMsgFlattenedSerializer::decode(data)?
                    };
                    return Ok(msg);
                }
                _ => {
                    demo_file.seek(SeekFrom::Current(cmd_header.size as i64))?;
                }
            }
        }
    }

    fn collect_unique_idents(flattened_serializer: &CsvcMsgFlattenedSerializer) -> Vec<String> {
        let mut set = HashSet::<String>::new();
        flattened_serializer.serializers.iter().for_each(|ser| {
            ser.fields_index.iter().for_each(|field_index| {
                if let Some(field) = flattened_serializer.fields.get(*field_index as usize) {
                    if let Some(var_type_sym) = field.var_type_sym {
                        if let Some(var_type) =
                            flattened_serializer.symbols.get(var_type_sym as usize)
                        {
                            Tokenizer::new(var_type.as_str()).for_each(|token| {
                                if let Token::Ident(ident) = token {
                                    set.insert(ident.to_string());
                                }
                            });
                        }
                    }
                }
            });
        });
        set.into_iter().collect()
    }

    pub(super) fn build() -> super::Result<()> {
        let flattened_serializer = get_flattened_serializer()?;
        let unique_idents = collect_unique_idents(&flattened_serializer);

        let out_dir = std::env::var("OUT_DIR")?;
        let out_file = std::path::Path::new(&out_dir).join("var_type.rs");
        string_cache_codegen::AtomType::new("var_type::IdentAtom", "var_type_ident_atom!")
            .atoms(unique_idents)
            .write_to_file(&out_file)?;

        Ok(())
    }
}

fn main() -> Result<()> {
    var_type::build()?;

    Ok(())
}
