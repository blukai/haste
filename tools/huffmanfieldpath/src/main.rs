// this tool helped to figure out stuff about huffman coding which is used to
// encode field paths. it can generate table that will include ids that can be
// used for "static" lookups (which is quicker then walking the tree or looking
// up stuff in the hash map). it can also generate a dot representation that can
// be used to visualize tree in graph formant.
//
// it is not so cool to blindly rely on someone else's undocumented findings; it
// feels better to build a deeper understanding of what's going on.

use std::{cmp::Reverse, collections::BinaryHeap, fmt::Debug, io::Write};

struct Op {
    name: &'static str,
    weight: u32,
}

// NOTE: names and weights are stolen from butterfly; in ghidra it is possible
// to find names (in string search); it is also possible to find where huffman
// tree is supposedly being constructed (search for one of the weights (decimal
// format)), but it is not clear how weights and names are being associalted /
// mapped together.
//
// void FUN_002ea630(long **param_1) {
//   ...
//   *(undefined4 *)param_1[2] = 0x8daf; // 36271
//   *(undefined4 *)((long)param_1[2] + 0x9c) = 0x6382; // 25474
//  ...
// }
//
// TODO: figure out how to map op names to weights in disassembly.

const OPS: [Op; 40] = [
    Op {
        name: "PlusOne",
        weight: 36271,
    },
    Op {
        name: "PlusTwo",
        weight: 10334,
    },
    Op {
        name: "PlusThree",
        weight: 1375,
    },
    Op {
        name: "PlusFour",
        weight: 646,
    },
    Op {
        name: "PlusN",
        weight: 4128,
    },
    Op {
        name: "PushOneLeftDeltaZeroRightZero",
        weight: 35,
    },
    Op {
        name: "PushOneLeftDeltaZeroRightNonZero",
        weight: 3,
    },
    Op {
        name: "PushOneLeftDeltaOneRightZero",
        weight: 521,
    },
    Op {
        name: "PushOneLeftDeltaOneRightNonZero",
        weight: 2942,
    },
    Op {
        name: "PushOneLeftDeltaNRightZero",
        weight: 560,
    },
    Op {
        name: "PushOneLeftDeltaNRightNonZero",
        weight: 471,
    },
    Op {
        name: "PushOneLeftDeltaNRightNonZeroPack6Bits",
        weight: 10530,
    },
    Op {
        name: "PushOneLeftDeltaNRightNonZeroPack8Bits",
        weight: 251,
    },
    Op {
        name: "PushTwoLeftDeltaZero",
        weight: 1,
    },
    Op {
        name: "PushTwoPack5LeftDeltaZero",
        weight: 1,
    },
    Op {
        name: "PushThreeLeftDeltaZero",
        weight: 1,
    },
    Op {
        name: "PushThreePack5LeftDeltaZero",
        weight: 1,
    },
    Op {
        name: "PushTwoLeftDeltaOne",
        weight: 1,
    },
    Op {
        name: "PushTwoPack5LeftDeltaOne",
        weight: 1,
    },
    Op {
        name: "PushThreeLeftDeltaOne",
        weight: 1,
    },
    Op {
        name: "PushThreePack5LeftDeltaOne",
        weight: 1,
    },
    Op {
        name: "PushTwoLeftDeltaN",
        weight: 1,
    },
    Op {
        name: "PushTwoPack5LeftDeltaN",
        weight: 1,
    },
    Op {
        name: "PushThreeLeftDeltaN",
        weight: 1,
    },
    Op {
        name: "PushThreePack5LeftDeltaN",
        weight: 1,
    },
    Op {
        name: "PushN",
        weight: 1,
    },
    Op {
        name: "PushNAndNonTopological",
        weight: 310,
    },
    Op {
        name: "PopOnePlusOne",
        weight: 2,
    },
    Op {
        name: "PopOnePlusN",
        weight: 1,
    },
    Op {
        name: "PopAllButOnePlusOne",
        weight: 1837,
    },
    Op {
        name: "PopAllButOnePlusN",
        weight: 149,
    },
    Op {
        name: "PopAllButOnePlusNPack3Bits",
        weight: 300,
    },
    Op {
        name: "PopAllButOnePlusNPack6Bits",
        weight: 634,
    },
    Op {
        name: "PopNPlusOne",
        weight: 1,
    },
    Op {
        name: "PopNPlusN",
        weight: 1,
    },
    Op {
        name: "PopNAndNonTopographical",
        weight: 1,
    },
    Op {
        name: "NonTopoComplex",
        weight: 76,
    },
    Op {
        name: "NonTopoPenultimatePlusOne",
        weight: 271,
    },
    Op {
        name: "NonTopoComplexPack4Bits",
        weight: 99,
    },
    Op {
        name: "FieldPathEncodeFinish",
        weight: 25474,
    },
];

// based on https://github.com/Lakret/huffman-rs/blob/4e2f759e2ca384108e5c95bc9cd365fad1d48364/src/huffman.rs
// NOTE: Tree does not have an A: Allocator generic param because it's not
// supposed to be used in real code, but for code generation.
#[derive(Debug, PartialEq, Eq)]
enum HuffmanTree<V> {
    Leaf {
        weight: u32,
        value: V,
    },
    Node {
        weight: u32,
        left: Box<HuffmanTree<V>>,
        right: Box<HuffmanTree<V>>,
    },
}

impl<V: Clone> HuffmanTree<V> {
    fn get_weight(&self) -> u32 {
        match self {
            Self::Leaf { weight, .. } => *weight,
            Self::Node { weight, .. } => *weight,
        }
    }
}

impl<V: Clone + Eq> Ord for HuffmanTree<V> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.get_weight().cmp(&other.get_weight())
    }
}

impl<V: Clone + Eq> PartialOrd for HuffmanTree<V> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn make_huffman_tree() -> HuffmanTree<usize> {
    let mut bh = BinaryHeap::with_capacity(OPS.len());
    for (i, fop) in OPS.iter().enumerate() {
        bh.push(Reverse(HuffmanTree::Leaf {
            weight: fop.weight,
            value: i,
        }));
    }

    while bh.len() > 1 {
        let left = bh.pop().unwrap().0;
        let right = bh.pop().unwrap().0;
        let merged_node = HuffmanTree::Node {
            weight: left.get_weight() + right.get_weight(),
            left: Box::new(left),
            right: Box::new(right),
        };
        bh.push(Reverse(merged_node));
    }

    bh.pop().unwrap().0
}

// ----

fn walk_huffman_tree_and_write_table<W: Write>(huffman_tree: &HuffmanTree<usize>, w: &mut W) {
    fn walk<W: Write>(huffman_tree: &HuffmanTree<usize>, w: &mut W, id: u32) {
        match huffman_tree {
            HuffmanTree::Leaf { weight, value: idx } => {
                writeln!(
                    w,
                    "{:>38} | {:>6} | {:>6}",
                    OPS[*idx as usize].name, weight, id
                )
                .unwrap();
            }
            HuffmanTree::Node { left, right, .. } => {
                walk(right, w, (id << 1) | 1);
                walk(left, w, (id << 1) | 0);
            }
        }
    }

    writeln!(w, "{:>38} | weight |     id", "name").unwrap();
    walk(huffman_tree, w, 0);
}

fn walk_huffman_tree_and_write_digraph<W: Write>(huffman_tree: &HuffmanTree<usize>, w: &mut W) {
    fn walk<W: Write>(huffman_tree: &HuffmanTree<usize>, w: &mut W, id: u32) {
        match huffman_tree {
            HuffmanTree::Leaf {
                value: idx, weight, ..
            } => {
                writeln!(
                    w,
                    "  {} [label=\"{}\\nweight {}, id {}\"];",
                    id, OPS[*idx as usize].name, weight, id
                )
                .unwrap();
            }
            HuffmanTree::Node { left, right, .. } => {
                writeln!(w, "  {} [label=\"\"];", id).unwrap();
                writeln!(w, "  {} -> {};", id, (id << 1) | 0).unwrap();
                writeln!(w, "  {} -> {};", id, (id << 1) | 1).unwrap();

                walk(right, w, (id << 1) | 1);
                walk(left, w, (id << 1) | 0);
            }
        }
    }

    writeln!(w, "digraph HuffmanTree {{").unwrap();
    walk(huffman_tree, w, 0);
    writeln!(w, "}}").unwrap();
}

fn get_huffman_tree_depth<V>(huffman_tree: &HuffmanTree<V>) -> u32 {
    fn walk<V>(node: &HuffmanTree<V>, depth: u32) -> u32 {
        match node {
            HuffmanTree::Leaf { .. } => depth,
            HuffmanTree::Node { left, right, .. } => {
                let left_depth = walk(left, depth + 1);
                let right_depth = walk(right, depth + 1);
                left_depth.max(right_depth)
            }
        }
    }
    walk(huffman_tree, 0)
}

// ----

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1);
    if cmd.is_none() {
        eprintln!("usage: huffmanfieldpath <table|digraph>");
        std::process::exit(42);
    }

    let huffman_tree = make_huffman_tree();

    let stdout = std::io::stdout();
    let mut w = stdout.lock();

    match cmd.unwrap().as_str() {
        // table command is useful for constructing id lookup "table" (manually;
        // see fieldpath.rs).
        "table" => walk_huffman_tree_and_write_table(&huffman_tree, &mut w),
        // digraph command is fun xd. to get the visualisation run (graphviz
        // must be installed):
        // $ cargo run --bin huffman -- digraph | dot -Tpng | feh -
        "digraph" => walk_huffman_tree_and_write_digraph(&huffman_tree, &mut w),
        // depth command finds the depth of the huffman tree.
        "depth" => println!("depth: {}", get_huffman_tree_depth(&huffman_tree)),
        cmd => eprintln!("invalid command: {}", cmd),
    }
}
