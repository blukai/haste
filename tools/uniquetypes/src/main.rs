// uniquetypes tool can be used to get a list of unique types that are present
// in the replay file.

use dota2protos::{self, EDemoCommands};
use muerta::{demofile::DemoFile, varint};
use prost::Message;
use std::{
    fs::File,
    io::{BufReader, SeekFrom},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let filepath = args.get(1);
    if filepath.is_none() {
        eprintln!("usage: uniquetypes <filepath>");
        std::process::exit(42);
    }

    let file = File::open(filepath.unwrap())?;
    let file = BufReader::new(file);
    let mut demo_file = DemoFile::from_reader(BufReader::new(file));
    let _demo_header = demo_file.read_demo_header()?;

    let mut buf = vec![0u8; 2 * 1024 * 1024];
    loop {
        let cmd_header = demo_file.read_cmd_header()?;
        match cmd_header.command {
            // DemSendTables cmd is sent only once
            EDemoCommands::DemSendTables => {
                let flattened_serializer = {
                    let cmd = demo_file
                        .read_cmd::<dota2protos::CDemoSendTables>(&cmd_header, &mut buf)?;
                    let mut data = &cmd.data.expect("send tables data")[..];
                    let (_size, _count) = varint::read_uvarint32(&mut data)?;
                    dota2protos::CsvcMsgFlattenedSerializer::decode(data)?
                };

                let mut types = std::collections::HashSet::<String>::new();

                for serializer in flattened_serializer.serializers.into_iter() {
                    for field_index in serializer.fields_index.into_iter() {
                        let field = &flattened_serializer.fields[field_index as usize];
                        let resolve =
                            |v: i32| Some(String::from(&flattened_serializer.symbols[v as usize]));
                        let var_type = field.var_type_sym.and_then(resolve).expect("var type");
                        types.insert(var_type.clone());
                    }
                }

                let mut types = types.into_iter().collect::<Vec<String>>();
                types.sort();
                for typ in types {
                    println!("{}", typ);
                }

                break;
            }
            _ => {
                demo_file.seek(SeekFrom::Current(cmd_header.size as i64))?;
            }
        }
    }

    Ok(())
}
