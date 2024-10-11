use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

use anyhow::{Context as _, Result};
use haste::demofile::DemoFile;
use haste::entities::{deadlock_coord_from_cell, fkey_from_path, DeltaHeader, Entity};
use haste::fxhash;
use haste::parser::{Context, Parser, Visitor};

fn get_entity_coord(entity: &Entity, cell_key: &u64, vec_key: &u64) -> Option<f32> {
    let cell: u16 = entity.get_value(cell_key)?;
    let vec: f32 = entity.get_value(vec_key)?;
    let coord = deadlock_coord_from_cell(cell, vec);
    Some(coord)
}

fn get_entity_position(entity: &Entity) -> Option<[f32; 3]> {
    const CX: u64 = fkey_from_path(&["CBodyComponent", "m_cellX"]);
    const CY: u64 = fkey_from_path(&["CBodyComponent", "m_cellY"]);
    const CZ: u64 = fkey_from_path(&["CBodyComponent", "m_cellZ"]);

    const VX: u64 = fkey_from_path(&["CBodyComponent", "m_vecX"]);
    const VY: u64 = fkey_from_path(&["CBodyComponent", "m_vecY"]);
    const VZ: u64 = fkey_from_path(&["CBodyComponent", "m_vecZ"]);

    let x = get_entity_coord(entity, &CX, &VX)?;
    let y = get_entity_coord(entity, &CY, &VY)?;
    let z = get_entity_coord(entity, &CZ, &VZ)?;

    Some([x, y, z])
}

const DEADLOCK_PLAYERPAWN_ENTITY: u64 = fxhash::hash_bytes(b"CCitadelPlayerPawn");

#[derive(Default, Debug)]
struct MyVisitor {
    positions: HashMap<i32, [f32; 3]>,
}

impl MyVisitor {
    fn handle_player_pawn(&mut self, entity: &Entity) -> Result<()> {
        let position = get_entity_position(entity).expect("player pawn position");

        // TODO: get rid of hashmap, parser must supply a list of updated fields.
        match self.positions.entry(entity.index()) {
            Entry::Occupied(mut oe) => {
                let prev_position = oe.insert(position);
                if prev_position != position {
                    eprintln!(
                        "{} moved from {:?} to {:?}",
                        entity.index(),
                        prev_position,
                        position
                    );
                }
            }
            Entry::Vacant(ve) => {
                ve.insert(position);
            }
        };

        Ok(())
    }
}

impl Visitor for MyVisitor {
    fn on_entity(
        &mut self,
        _ctx: &Context,
        _delta_header: DeltaHeader,
        entity: &Entity,
    ) -> Result<()> {
        if entity.serializer_name_heq(DEADLOCK_PLAYERPAWN_ENTITY) {
            self.handle_player_pawn(entity)?;
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let filepath = args.get(1).context("usage: deadlock-position <filepath>")?;
    let file = File::open(filepath)?;
    let buf_reader = BufReader::new(file);
    let demo_file = DemoFile::start_reading(buf_reader)?;
    let mut parser = Parser::from_stream_with_visitor(demo_file, MyVisitor::default())?;
    parser.run_to_end()
}
