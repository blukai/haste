// this tool helped to figure out stuff about huffman coding which is used to
// encode field paths. it can generate table that will include ids that can be
// used for "static" lookups (which is quicker then walking the tree or looking
// up stuff in the hash map). it can also generate a dot representation that can
// be used to visualize tree in graph formant.
//
// it is not so cool to blindly rely on someone else's undocumented findings; it
// feels better to build a deeper understanding of what's going on.

use std::{cmp::Ordering, collections::BinaryHeap, fmt::Debug};

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

#[derive(Debug)]
struct Op {
    name: &'static str,
    weight: usize,
}

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

#[derive(Debug)]
enum Huffman<Value: Debug> {
    Leaf {
        weight: usize,
        num: usize,
        value: Value,
    },
    Node {
        weight: usize,
        num: usize,
        left: Box<Huffman<Value>>,
        right: Box<Huffman<Value>>,
    },
}

impl<Value: Debug> Huffman<Value> {
    fn weight(&self) -> usize {
        match self {
            Self::Node { weight, .. } => *weight,
            Self::Leaf { weight, .. } => *weight,
        }
    }
    fn num(&self) -> usize {
        match self {
            Self::Node { num, .. } => *num,
            Self::Leaf { num, .. } => *num,
        }
    }
}

impl<Value: Debug> Ord for Huffman<Value> {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.weight() == other.weight() {
            self.num().cmp(&other.num())
        } else {
            other.weight().cmp(&self.weight())
        }
    }
}

impl<Value: Debug> PartialOrd for Huffman<Value> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<Value: Debug> PartialEq for Huffman<Value> {
    fn eq(&self, other: &Self) -> bool {
        self.weight() == other.weight() && self.num() == other.num()
    }
}

impl<Value: Debug> Eq for Huffman<Value> {}

fn make_huffman() -> Huffman<&'static Op> {
    // Valve's Huffman-Tree uses a variation which takes the node number into
    // account
    let mut num = 0;

    let mut bh = BinaryHeap::new();

    for op in OPS.iter() {
        let leaf = Huffman::Leaf {
            weight: op.weight,
            num,
            value: op,
        };
        bh.push(leaf);
        num += 1;
    }

    while bh.len() > 1 {
        let left = bh.pop().unwrap();
        let right = bh.pop().unwrap();
        let node = Huffman::Node {
            weight: left.weight() + right.weight(),
            num,
            left: Box::new(left),
            right: Box::new(right),
        };
        bh.push(node);
        num += 1;
    }

    bh.pop().unwrap()
}

fn print_table(huffman: &Huffman<&'static Op>) {
    struct Leaf {
        op: &'static Op,
        id: usize,
        depth: usize,
    }
    let mut leafs: Vec<Leaf> = Vec::new();

    fn walk(huffman: &Huffman<&'static Op>, leafs: &mut Vec<Leaf>, id: usize, depth: usize) {
        match huffman {
            Huffman::Leaf { value: op, .. } => {
                leafs.push(Leaf { op, id, depth });
            }
            Huffman::Node { left, right, .. } => {
                walk(right, leafs, (id << 1) | 1, depth + 1);
                walk(left, leafs, id << 1, depth + 1);
            }
        }
    }

    walk(huffman, &mut leafs, 0, 0);

    println!("{:>38} | weight |     id | depth", "name");
    leafs.sort_by(|a, b| a.id.partial_cmp(&b.id).unwrap());
    leafs.iter().for_each(|leaf| {
        println!(
            "{:>38} | {:>6} | {:>6} | {:>5}",
            leaf.op.name, leaf.op.weight, leaf.id, leaf.depth
        );
    });
}

fn print_dot(huffman: &Huffman<&'static Op>) {
    fn walk(huffman: &Huffman<&'static Op>, id: usize, depth: usize) {
        match huffman {
            Huffman::Leaf { value, weight, .. } => {
                println!(
                    "  {} [label=\"{}\\nweight {}, id {}, depth {}\"];",
                    id, value.name, weight, id, depth
                );
            }
            Huffman::Node { left, right, .. } => {
                println!("  {} [label=\"\"];", id);
                println!("  {} -> {};", id, (id << 1) | 1);
                println!("  {} -> {};", id, id << 1);

                walk(right, (id << 1) | 1, depth + 1);
                walk(left, id << 1, depth + 1);
            }
        }
    }

    println!("digraph Huffman {{");
    walk(huffman, 0, 0);
    println!("}}");
}

fn print_depth(huffman: &Huffman<&'static Op>) {
    fn walk(node: &Huffman<&'static Op>, depth: usize) -> usize {
        match node {
            Huffman::Leaf { .. } => depth,
            Huffman::Node { left, right, .. } => {
                let left_depth = walk(left, depth + 1);
                let right_depth = walk(right, depth + 1);
                left_depth.max(right_depth)
            }
        }
    }
    let depth = walk(huffman, 0);
    println!("{}", depth);
}

// // ----

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1);
    if cmd.is_none() {
        eprintln!("usage: huffmanfieldpath <table|dot|depth>");
        std::process::exit(42);
    }

    let huffman = make_huffman();

    match cmd.unwrap().as_str() {
        // table command is useful for constructing id lookup "table" (manually;
        // see fieldpath.rs).
        "table" => print_table(&huffman),
        // dot command is fun xd. to get the visualisation run (graphviz
        // must be installed):
        // $ cargo run --bin huffmanfieldpath -- dot | dot -Tpng | feh -
        "dot" => print_dot(&huffman),
        // depth command finds the depth of the huffman.
        "depth" => print_depth(&huffman),
        cmd => eprintln!("invalid command: {}", cmd),
    }
}
