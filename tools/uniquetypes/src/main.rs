use std::collections::HashSet;
use std::fs::File;
use std::io::BufReader;

use anyhow::{Context as _, Result};
use dungers::varint;
use haste::demofile::DemoFile;
use haste::demostream::DemoStream;
use haste::valveprotos::common::{CDemoSendTables, CsvcMsgFlattenedSerializer, EDemoCommands};
use haste_vartype::{TokenKind, Tokenizer};
use prost::Message;

fn resolve_sym(
    flattened_serializer: &CsvcMsgFlattenedSerializer,
    index: Option<&i32>,
) -> Option<String> {
    let index = index.cloned()? as usize;
    flattened_serializer.symbols.get(index).cloned()
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let filepath = args.get(1).context("usage: uniquetypes <filepath>")?;

    let file = File::open(filepath)?;
    let buf_reader = BufReader::new(file);
    let mut demo_file = DemoFile::start_reading(buf_reader)?;

    let send_tables = loop {
        let cmd_header = demo_file.read_cmd_header()?;
        assert!(cmd_header.tick <= 0);
        if cmd_header.cmd == EDemoCommands::DemSendTables {
            let cmd_body = demo_file.read_cmd(&cmd_header)?;
            break CDemoSendTables::decode(cmd_body)?;
        } else {
            demo_file.skip_cmd(&cmd_header)?;
        }
    };

    let mut data = &send_tables.data.unwrap_or_default()[..];
    // skip useless size info
    let _ = varint::read_uvarint64(&mut data)?;
    let flattened_serializer = CsvcMsgFlattenedSerializer::decode(data)?;

    let mut var_type_idents: HashSet<String> = HashSet::new();
    let mut var_types: HashSet<String> = HashSet::new();
    let mut var_encoders: HashSet<String> = HashSet::new();

    for fs in &flattened_serializer.serializers {
        for field_index in fs.fields_index.iter().cloned() {
            let field = flattened_serializer
                .fields
                .get(field_index as usize)
                .context("invalid index, huh?")?;

            let var_type = resolve_sym(&flattened_serializer, field.var_type_sym.as_ref())
                .context("could not resolve var type sym")?;

            for token in Tokenizer::new(&var_type) {
                let token = token?;
                if let TokenKind::Ident(ident) = token.kind {
                    var_type_idents.insert(ident.to_string());
                }
            }

            var_types.insert(var_type);

            if let Some(var_encoder) =
                resolve_sym(&flattened_serializer, field.var_encoder_sym.as_ref())
            {
                var_encoders.insert(var_encoder);
            };
        }
    }

    eprintln!("----------------------");
    eprintln!("unique var type idents");
    eprintln!("----------------------");
    let mut var_type_idents = var_type_idents.into_iter().collect::<Vec<String>>();
    var_type_idents.sort();
    var_type_idents.iter().for_each(|var_type_ident| {
        eprintln!("{var_type_ident}");
    });

    eprintln!("----------------");
    eprintln!("unique var types");
    eprintln!("----------------");
    let mut var_types = var_types.into_iter().collect::<Vec<String>>();
    var_types.sort();
    var_types.iter().for_each(|var_type| {
        eprintln!("{var_type}");
    });

    eprintln!("-------------------");
    eprintln!("unique var encoders");
    eprintln!("-------------------");
    let mut var_encoders = var_encoders.into_iter().collect::<Vec<String>>();
    var_encoders.sort();
    var_encoders.iter().for_each(|var_encoder| {
        eprintln!("{var_encoder}");
    });

    Ok(())
}
