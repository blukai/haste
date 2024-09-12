use haste::{
    entities::{make_field_key, DeltaHeader, Entity},
    fxhash,
    parser::{self, Context, Parser, Visitor},
};
use std::{
    collections::{hash_map::Entry, HashMap},
    fs::File,
    io::BufReader,
};

// TODO(blukai): figure out where to put those consts in haste crate. but note that values for dota
// are different!

// in replay that i'm fiddling with (3843940_683350910.dem) CBodyComponent.m_vecY of
// CCitadelPlayerPawn #4 at tick 111,077 is 1022.78125 and CBodyComponent.m_cellY is 36;
// at tick 111,080 CBodyComponent.m_vecY becomes 0.375 and CBodyComponent.m_cellY 38.
//
// thus CELL_BASEENTITY_ORIGIN_CELL_BITS = 10 so that 1 << 10 is 1024 - right?
// no, it's 9. why? i'm not exactly sure, but i would appreciate of somebody could explain.
//
// game/shared/shareddefs.h (adjusted)
const CELL_BASEENTITY_ORIGIN_CELL_BITS: u32 = 9;
// game/client/c_baseentity.cpp
const CELL_WIDTH: u32 = 1 << CELL_BASEENTITY_ORIGIN_CELL_BITS;

// CNPC_MidBoss (exactly in the middle of the map):
// CBodyComponent.m_cellX:uint16 = 32
// CBodyComponent.m_cellY:uint16 = 32
// CBodyComponent.m_cellZ:uint16 = 30
// CBodyComponent.m_vecX:CNetworkedQuantizedFloat = 0.0
// CBodyComponent.m_vecY:CNetworkedQuantizedFloat = 0.0
// CBodyComponent.m_vecZ:CNetworkedQuantizedFloat = 768.0
//
// from this it is safe to conclude that the actual grid is 64x64 which gives us
// MAX_COORD_INTEGER = CELL_WIDTH * 32. the same exact value that is defined in csgo.
//
// also CELL_COUNT can be computed as MAX_COORD_INTEGER * 2 / CELL_WIDTH.
//
// public/worldsize.h
const MAX_COORD_INTEGER: u32 = 16384;

/// given a cell and an offset in that cell, reconstruct the world coord
///
/// source: game/shared/cellcoord.h
fn coord_from_cell(cell_width: u32, max_coord: u32, cell: u16, vec: f32) -> f32 {
    let cell_pos = cell as u32 * cell_width;
    // nanitfi is r, what does it stand for in this context? (copypasting from valve)
    let r = (cell_pos as i32 - max_coord as i32) as f32 + vec;
    r
}

// CCitadelGameRulesProxy entity contains:
// m_pGameRules.m_vMinimapMins:Vector = [-8960.0, -8960.005, 0.0]
// m_pGameRules.m_vMinimapMaxs:Vector = [8960.0, 8960.0, 0.0]

fn get_entity_coord(entity: &Entity, cell_key: &u64, vec_key: &u64) -> Option<f32> {
    let cell: u16 = entity.get_value(cell_key)?;
    let vec: f32 = entity.get_value(vec_key)?;
    let coord = coord_from_cell(CELL_WIDTH, MAX_COORD_INTEGER, cell, vec);
    Some(coord)
}

fn get_entity_position(entity: &Entity) -> Option<[f32; 3]> {
    const CX: u64 = make_field_key(&["CBodyComponent", "m_cellX"]);
    const CY: u64 = make_field_key(&["CBodyComponent", "m_cellY"]);
    const CZ: u64 = make_field_key(&["CBodyComponent", "m_cellZ"]);

    const VX: u64 = make_field_key(&["CBodyComponent", "m_vecX"]);
    const VY: u64 = make_field_key(&["CBodyComponent", "m_vecY"]);
    const VZ: u64 = make_field_key(&["CBodyComponent", "m_vecZ"]);

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
    fn handle_player_pawn(&mut self, entity: &Entity) -> anyhow::Result<()> {
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

impl Visitor for &mut MyVisitor {
    fn on_entity(
        &mut self,
        _ctx: &Context,
        _delta_header: DeltaHeader,
        entity: &Entity,
    ) -> parser::Result<()> {
        if entity.serializer_name_heq(DEADLOCK_PLAYERPAWN_ENTITY) {
            self.handle_player_pawn(entity)?;
        }
        Ok(())
    }
}

fn main() -> parser::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let filepath = args.get(1);
    if filepath.is_none() {
        eprintln!("usage: deadlock-position <filepath>");
        std::process::exit(42);
    }

    let mut visitor = MyVisitor::default();

    let file = File::open(filepath.unwrap())?;
    let buf_reader = BufReader::new(file);
    let mut parser = Parser::from_reader_with_visitor(buf_reader, &mut visitor)?;
    parser.run_to_end()
}
