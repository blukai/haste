use anyhow::anyhow;
use haste_dota2::{
    deflat::var_type::{Token, Tokenizer},
    flattenedserializers::FlattenedSerializerContainer,
    parser::{ControlFlow, NopVisitor, Parser},
    protos::EDemoCommands,
};
use std::{
    collections::HashSet,
    fs::File,
    io::{BufReader, Read, Seek},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

// ----

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

fn collect_unique_var_types(serializers: &FlattenedSerializerContainer) -> Vec<String> {
    let mut tmp: HashSet<String> = HashSet::new();
    serializers.values().for_each(|fs| {
        fs.fields.iter().for_each(|f| {
            tmp.insert(f.var_type.str.to_string());
        });
    });
    tmp.into_iter().collect()
}

fn collect_unique_var_type_idents(serializers: &FlattenedSerializerContainer) -> Vec<String> {
    let mut tmp = HashSet::<String>::new();
    serializers.values().for_each(|fs| {
        fs.fields.iter().for_each(|f| {
            Tokenizer::new(&f.var_type.str).for_each(|token| {
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

    parse_to_serializers(&mut parser)?;
    let serializers = parser
        .serializers()
        .ok_or_else(|| anyhow!("could not get flattened serializer"))?;
    eprintln!("----------------");
    eprintln!("unique var types");
    eprintln!("----------------");
    let mut var_types = collect_unique_var_types(serializers);
    var_types.sort();
    var_types.iter().for_each(|var_type| {
        eprintln!("{var_type}");
    });

    eprintln!("----------------------");
    eprintln!("unique var type idents");
    eprintln!("----------------------");
    let mut var_type_idents = collect_unique_var_type_idents(serializers);
    var_type_idents.sort();
    var_type_idents.iter().for_each(|var_type_ident| {
        eprintln!("{var_type_ident}");
    });

    // ----

    Ok(())
}

// TODO: investigate arena.alloc_from_iter (specifically arena) in rust source
// code.
