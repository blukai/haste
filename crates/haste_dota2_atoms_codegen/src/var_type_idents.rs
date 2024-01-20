use anyhow::anyhow;
use haste_dota2::{
    deflat::var_type::{Token, Tokenizer},
    flattenedserializers::FlattenedSerializerContainer,
    parser::{ControlFlow, NopVisitor, Parser},
    protos::EDemoCommands,
};
use std::{
    collections::HashSet,
    io::{Read, Seek},
    path::Path,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn parse_to_serializers<R: Read + Seek>(parser: &mut Parser<R, NopVisitor>) -> Result<()> {
    parser.reset()?;
    parser.run(|notnotself, cmd_header| {
        if notnotself.serializers().is_some() {
            return Ok(ControlFlow::Break);
        }
        if cmd_header.command == EDemoCommands::DemSendTables {
            return Ok(ControlFlow::HandleCmd);
        }
        Ok(ControlFlow::SkipCmd)
    })
}

fn collect_unique_var_type_idents(serializers: &FlattenedSerializerContainer) -> Vec<String> {
    let mut tmp = HashSet::<String>::new();
    serializers.values().for_each(|fs| {
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

pub fn build<R: Read + Seek>(parser: &mut Parser<R, NopVisitor>, out_path: &Path) -> Result<()> {
    parse_to_serializers(parser)?;
    let serializers = parser
        .serializers()
        .ok_or_else(|| anyhow!("could not get flattened serializer"))?;
    let unique_var_type_idents = collect_unique_var_type_idents(serializers);

    string_cache_codegen::AtomType::new("var_type_ident::VarTypeIdentAtom", "var_type_ident_atom!")
        .atoms(unique_var_type_idents)
        .write_to_file(&out_path.join("src/var_type_ident.rs"))?;

    Ok(())
}
