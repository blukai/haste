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
    fs::File,
    io::{Read, Seek, SeekFrom},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

struct Parser<R: Read + Seek> {
    demo_file: DemoFile<R>,
    buf: Vec<u8>,
    string_tables: StringTables,
    instance_baseline: InstanceBaseline,
    flattened_serializers: FlattenedSerializers,
    entity_classes: EntityClasses,
    entities: Entities,
}

impl<R: Read + Seek> Parser<R> {
    fn from_reader(r: R) -> Result<Self> {
        let mut demo_file = DemoFile::from_reader(r);
        // TODO: validate demo header
        let _demo_header = demo_file.read_demo_header()?;

        Ok(Self {
            demo_file,
            // 2mb
            buf: vec![0; 1024 * 1024 * 2],
            string_tables: StringTables::new(),
            instance_baseline: InstanceBaseline::new(),
            flattened_serializers: FlattenedSerializers::new(),
            entity_classes: EntityClasses::new(),
            entities: Entities::new(),
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

            match command {
                c if c == SvcMessages::SvcCreateStringTable as u32 => {
                    let svcmsg = protos::CsvcMsgCreateStringTable::decode(&buf[..])?;
                    self.handle_svcmsg_create_string_table(svcmsg)?;
                }

                c if c == SvcMessages::SvcUpdateStringTable as u32 => {
                    let svcmsg = protos::CsvcMsgUpdateStringTable::decode(&buf[..])?;
                    self.handle_svcmsg_update_string_table(svcmsg)?;
                }

                c if c == SvcMessages::SvcPacketEntities as u32 => {
                    let svcmsg = protos::CsvcMsgPacketEntities::decode(&buf[..])?;
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
            snap::raw::Decoder::new().decompress_vec(svcmsg.string_data())?
        } else {
            svcmsg.string_data().to_vec()
        };
        string_table.parse_update(&mut BitReader::new(&string_data), svcmsg.num_entries())?;

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
    let file = File::open("./fixtures/7116662198_1379602574.dem")?;
    let mut parser = Parser::from_reader(file)?;
    parser.run()?;
    println!("done!");
    Ok(())
}
