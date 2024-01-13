use anyhow::anyhow;
use haste_dota2::{
    dota2_deflat::var_type::{Token, Tokenizer},
    dota2_protos::EDemoCommands,
    flattenedserializers::FlattenedSerializers,
    parser::{ControlFlow, NopVisitor, Parser},
};
use std::{
    collections::HashSet,
    fs::File,
    io::{BufReader, Read, Seek},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

// ----

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

fn collect_unique_var_types(flattened_serializers: &FlattenedSerializers) -> Vec<String> {
    let mut tmp: HashSet<String> = HashSet::new();
    flattened_serializers.values().for_each(|fs| {
        fs.fields.iter().for_each(|f| {
            tmp.insert(f.var_type.to_string());
        });
    });
    tmp.into_iter().collect()
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

// ----

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let filepath = args.get(1);
    if filepath.is_none() {
        eprintln!("usage: uniquetypes <filepath>");
        std::process::exit(42);
    }

    let file = File::open(filepath.unwrap())?;
    let buf_reader = BufReader::new(file);

    let mut parser = Parser::from_reader(buf_reader)?;

    // ----

    parse_to_flattened_serializers(&mut parser)?;
    let flattened_serializers = parser
        .flattened_serializers()
        .ok_or_else(|| anyhow!("could not get flattened serializer"))?;
    eprintln!("----------------");
    eprintln!("unique var types");
    eprintln!("----------------");
    let mut var_types = collect_unique_var_types(flattened_serializers);
    var_types.sort();
    var_types.iter().for_each(|var_type| {
        eprintln!("{var_type}");
    });

    eprintln!("----------------------");
    eprintln!("unique var type idents");
    eprintln!("----------------------");
    let mut var_type_idents = collect_unique_var_type_idents(flattened_serializers);
    var_type_idents.sort();
    var_type_idents.iter().for_each(|var_type_ident| {
        eprintln!("{var_type_ident}");
    });

    // ----

    Ok(())
}

// TODO: investigate arena.alloc_from_iter (specifically arena) in rust source
// code.
