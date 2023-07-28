#![feature(allocator_api)]

use muerta::{
    bitbuf::BitReader,
    demofile::{CmdHeader, DemoFile},
    entities::Entities,
    entityclasses::EntityClasses,
    flattenedserializers::FlattenedSerializers,
    instancebaseline::{InstanceBaseline, INSTANCE_BASELINE_TABLE_NAME},
    protos::{self, EDemoCommands, SvcMessages},
    stringtables::StringTables,
};
use prost::Message;
use std::{
    alloc::Allocator,
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

struct Parser<R: Read + Seek, A: Allocator + Clone> {
    demo_file: DemoFile<R>,
    buf: Vec<u8>,
    string_tables: StringTables<A>,
    instance_baseline: InstanceBaseline<A>,
    flattened_serializers: FlattenedSerializers<A>,
    entity_classes: EntityClasses<A>,
    entities: Entities<A>,
}

impl<R: Read + Seek, A: Allocator + Clone> Parser<R, A> {
    fn from_reader_in(r: R, alloc: A) -> Result<Self> {
        let mut demo_file = DemoFile::from_reader(r);
        // TODO: validate demo header
        let _demo_header = demo_file.read_demo_header()?;

        Ok(Self {
            demo_file,
            // 2mb
            buf: vec![0; 1024 * 1024 * 2],
            string_tables: StringTables::new_in(alloc.clone()),
            instance_baseline: InstanceBaseline::new_in(alloc.clone()),
            flattened_serializers: FlattenedSerializers::new_in(alloc.clone()),
            entity_classes: EntityClasses::new_in(alloc.clone()),
            entities: Entities::new_in(alloc),
        })
    }

    fn run(&mut self) -> Result<()> {
        loop {
            let cmd_header = self.demo_file.read_cmd_header()?;
            if cmd_header.command == EDemoCommands::DemStop {
                break;
            }
            self.handle_cmd(cmd_header)?;
        }
        Ok(())
    }

    fn handle_cmd(&mut self, cmd_header: CmdHeader) -> Result<()> {
        match cmd_header.command {
            EDemoCommands::DemPacket | EDemoCommands::DemSignonPacket => {
                let cmd = self
                    .demo_file
                    .read_cmd::<protos::CDemoPacket>(&cmd_header, &mut self.buf)?;
                self.handle_cmd_packet(cmd)?;
            }

            EDemoCommands::DemSendTables => {
                let cmd = self
                    .demo_file
                    .read_cmd::<protos::CDemoSendTables>(&cmd_header, &mut self.buf)?;
                self.flattened_serializers.parse(cmd)?;
            }

            EDemoCommands::DemClassInfo => {
                let cmd = self
                    .demo_file
                    .read_cmd::<protos::CDemoClassInfo>(&cmd_header, &mut self.buf)?;
                self.entity_classes.parse(cmd);
            }

            _ => {
                self.demo_file
                    .seek(SeekFrom::Current(cmd_header.size as i64))?;
            }
        }

        Ok(())
    }

    fn handle_cmd_packet(&mut self, proto: protos::CDemoPacket) -> Result<()> {
        let data = proto.data.expect("demo packet data");
        let mut br = BitReader::new(&data);

        while br.get_num_bits_left() > 8 {
            let command = br.read_ubitvar()?;
            let size = br.read_uvarint32()? as usize;

            let buf = &mut self.buf[..size];
            br.read_bytes(buf)?;
            let data: &_ = buf;

            match command {
                c if c == SvcMessages::SvcCreateStringTable as u32 => {
                    let svcmsg = protos::CsvcMsgCreateStringTable::decode(data)?;
                    self.handle_svcmsg_create_string_table(svcmsg)?;
                }

                c if c == SvcMessages::SvcUpdateStringTable as u32 => {
                    let svcmsg = protos::CsvcMsgUpdateStringTable::decode(data)?;
                    self.handle_svcmsg_update_string_table(svcmsg)?;
                }

                c if c == SvcMessages::SvcPacketEntities as u32 => {
                    let svcmsg = protos::CsvcMsgPacketEntities::decode(data)?;
                    self.handle_svcmsg_packet_entities(svcmsg)?;
                }

                _ => {}
            }
        }

        Ok(())
    }

    fn handle_svcmsg_create_string_table(
        &mut self,
        svcmsg: protos::CsvcMsgCreateStringTable,
    ) -> Result<()> {
        let string_table = self.string_tables.create_string_table_mut(
            svcmsg.name(),
            svcmsg.user_data_fixed_size(),
            svcmsg.user_data_size(),
            svcmsg.user_data_size_bits(),
            svcmsg.flags(),
            svcmsg.using_varint_bitcounts(),
        )?;

        let string_data = if svcmsg.data_compressed() {
            let sd = svcmsg.string_data();
            let decompress_len = snap::raw::decompress_len(sd)?;
            snap::raw::Decoder::new().decompress(sd, &mut self.buf)?;
            &self.buf[..decompress_len]
        } else {
            svcmsg.string_data()
        };
        string_table.parse_update(&mut BitReader::new(string_data), svcmsg.num_entries())?;

        if string_table.name.eq(INSTANCE_BASELINE_TABLE_NAME) {
            self.instance_baseline.update(string_table)?;
        }

        Ok(())
    }

    fn handle_svcmsg_update_string_table(
        &mut self,
        svcmsg: protos::CsvcMsgUpdateStringTable,
    ) -> Result<()> {
        let string_table = self
            .string_tables
            .get_table_mut(svcmsg.table_id.expect("table id") as usize)
            .expect("table");

        string_table.parse_update(
            &mut BitReader::new(svcmsg.string_data()),
            svcmsg.num_changed_entries(),
        )?;

        if string_table.name.eq(INSTANCE_BASELINE_TABLE_NAME) {
            self.instance_baseline.update(string_table)?;
        }

        Ok(())
    }

    fn handle_svcmsg_packet_entities(
        &mut self,
        svcmsg: protos::CsvcMsgPacketEntities,
    ) -> Result<()> {
        self.entities.read_packet_entities(
            svcmsg,
            &self.entity_classes,
            &self.instance_baseline,
            &self.flattened_serializers,
        )?;
        Ok(())
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let filepath = args.get(1);
    if filepath.is_none() {
        eprintln!("usage: parseentities <filepath>");
        std::process::exit(42);
    }

    #[cfg(feature = "bumpalo")]
    let bump = bumpalo::Bump::with_capacity(1024 * 1024 * 1024 * 2);
    #[cfg(feature = "bumpalo")]
    let alloc = &bump;

    #[cfg(not(feature = "bumpalo"))]
    let alloc = std::alloc::Global;

    let file = File::open(filepath.unwrap())?;
    let file = BufReader::new(file);
    let mut parser = Parser::from_reader_in(file, alloc)?;
    parser.run()?;
    println!("done!");

    Ok(())
}
