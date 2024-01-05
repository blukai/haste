use anyhow::anyhow;
use haste_dota2::{
    dota2_deflat::var_type::{Token, Tokenizer},
    dota2_protos::EDemoCommands,
    flattenedserializers::FlattenedSerializers,
    parser::{ControlFlow, NopVisitor, Parser},
};
use std::{
    collections::HashSet,
    io::{Read, Seek},
    path::PathBuf,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn parse_to_flattened_serializers<R: Read + Seek>(
    parser: &mut Parser<R, NopVisitor>,
) -> Result<()> {
    parser.reset()?;
    parser.run(|notnotself, cmd_header| {
        if notnotself.flattened_serializers().is_some() {
            return Ok(ControlFlow::Break);
        }
        if cmd_header.command == EDemoCommands::DemSendTables {
            return Ok(ControlFlow::HandleCmd);
        }
        Ok(ControlFlow::SkipCmd)
    })
}

fn collect_unique_var_type_idents(flattened_serializers: &FlattenedSerializers) -> Vec<String> {
    let mut tmp = HashSet::<String>::new();
    flattened_serializers.values().for_each(|fs| {
        fs.fields.iter().for_each(|f| {
            Tokenizer::new(&f.var_type).for_each(|token| {
                if let Token::Ident(ident) = token {
                    tmp.insert(ident.to_string());
                }
            });
        });
    });
    tmp.into_iter().collect()
}

pub fn build<R: Read + Seek>(parser: &mut Parser<R, NopVisitor>, out_path: &PathBuf) -> Result<()> {
    parse_to_flattened_serializers(parser)?;
    let flattened_serializers = parser
        .flattened_serializers()
        .ok_or_else(|| anyhow!("could not get flattened serializer"))?;
    let unique_var_type_idents = collect_unique_var_type_idents(flattened_serializers);

    string_cache_codegen::AtomType::new("var_type_ident::VarTypeIdentAtom", "var_type_ident_atom!")
        .atoms(unique_var_type_idents)
        .write_to_file(&out_path.join("src/var_type_ident.rs"))?;

    Ok(())
}
