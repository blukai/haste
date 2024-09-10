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
struct FieldOpDescriptor {
    name: &'static str,
    weight: usize,
}

const FIELDOP_DESCRIPTORS: [FieldOpDescriptor; 40] = [
    FieldOpDescriptor {
        name: "PlusOne",
        weight: 36271,
    },
    FieldOpDescriptor {
        name: "PlusTwo",
        weight: 10334,
    },
    FieldOpDescriptor {
        name: "PlusThree",
        weight: 1375,
    },
    FieldOpDescriptor {
        name: "PlusFour",
        weight: 646,
    },
    FieldOpDescriptor {
        name: "PlusN",
        weight: 4128,
    },
    FieldOpDescriptor {
        name: "PushOneLeftDeltaZeroRightZero",
        weight: 35,
    },
    FieldOpDescriptor {
        name: "PushOneLeftDeltaZeroRightNonZero",
        weight: 3,
    },
    FieldOpDescriptor {
        name: "PushOneLeftDeltaOneRightZero",
        weight: 521,
    },
    FieldOpDescriptor {
        name: "PushOneLeftDeltaOneRightNonZero",
        weight: 2942,
    },
    FieldOpDescriptor {
        name: "PushOneLeftDeltaNRightZero",
        weight: 560,
    },
    FieldOpDescriptor {
        name: "PushOneLeftDeltaNRightNonZero",
        weight: 471,
    },
    FieldOpDescriptor {
        name: "PushOneLeftDeltaNRightNonZeroPack6Bits",
        weight: 10530,
    },
    FieldOpDescriptor {
        name: "PushOneLeftDeltaNRightNonZeroPack8Bits",
        weight: 251,
    },
    FieldOpDescriptor {
        name: "PushTwoLeftDeltaZero",
        weight: 1,
    },
    FieldOpDescriptor {
        name: "PushTwoPack5LeftDeltaZero",
        weight: 1,
    },
    FieldOpDescriptor {
        name: "PushThreeLeftDeltaZero",
        weight: 1,
    },
    FieldOpDescriptor {
        name: "PushThreePack5LeftDeltaZero",
        weight: 1,
    },
    FieldOpDescriptor {
        name: "PushTwoLeftDeltaOne",
        weight: 1,
    },
    FieldOpDescriptor {
        name: "PushTwoPack5LeftDeltaOne",
        weight: 1,
    },
    FieldOpDescriptor {
        name: "PushThreeLeftDeltaOne",
        weight: 1,
    },
    FieldOpDescriptor {
        name: "PushThreePack5LeftDeltaOne",
        weight: 1,
    },
    FieldOpDescriptor {
        name: "PushTwoLeftDeltaN",
        weight: 1,
    },
    FieldOpDescriptor {
        name: "PushTwoPack5LeftDeltaN",
        weight: 1,
    },
    FieldOpDescriptor {
        name: "PushThreeLeftDeltaN",
        weight: 1,
    },
    FieldOpDescriptor {
        name: "PushThreePack5LeftDeltaN",
        weight: 1,
    },
    FieldOpDescriptor {
        name: "PushN",
        weight: 1,
    },
    FieldOpDescriptor {
        name: "PushNAndNonTopological",
        weight: 310,
    },
    FieldOpDescriptor {
        name: "PopOnePlusOne",
        weight: 2,
    },
    FieldOpDescriptor {
        name: "PopOnePlusN",
        weight: 1,
    },
    FieldOpDescriptor {
        name: "PopAllButOnePlusOne",
        weight: 1837,
    },
    FieldOpDescriptor {
        name: "PopAllButOnePlusN",
        weight: 149,
    },
    FieldOpDescriptor {
        name: "PopAllButOnePlusNPack3Bits",
        weight: 300,
    },
    FieldOpDescriptor {
        name: "PopAllButOnePlusNPack6Bits",
        weight: 634,
    },
    FieldOpDescriptor {
        name: "PopNPlusOne",
        weight: 1,
    },
    FieldOpDescriptor {
        name: "PopNPlusN",
        weight: 1,
    },
    FieldOpDescriptor {
        name: "PopNAndNonTopographical",
        weight: 1,
    },
    FieldOpDescriptor {
        name: "NonTopoComplex",
        weight: 76,
    },
    FieldOpDescriptor {
        name: "NonTopoPenultimatePlusOne",
        weight: 271,
    },
    FieldOpDescriptor {
        name: "NonTopoComplexPack4Bits",
        weight: 99,
    },
    FieldOpDescriptor {
        name: "FieldPathEncodeFinish",
        weight: 25474,
    },
];

#[derive(Debug)]
enum Node<T: Debug> {
    Leaf {
        weight: usize,
        num: usize,
        value: T,
    },
    Branch {
        weight: usize,
        num: usize,
        left: Box<Node<T>>,
        right: Box<Node<T>>,
    },
}

impl<T: Debug> Node<T> {
    fn weight(&self) -> usize {
        match self {
            Self::Leaf { weight, .. } => *weight,
            Self::Branch { weight, .. } => *weight,
        }
    }

    fn num(&self) -> usize {
        match self {
            Self::Leaf { num, .. } => *num,
            Self::Branch { num, .. } => *num,
        }
    }
}

impl<T: Debug> Ord for Node<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.weight() == other.weight() {
            self.num().cmp(&other.num())
        } else {
            other.weight().cmp(&self.weight())
        }
    }
}

impl<T: Debug> PartialOrd for Node<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Debug> PartialEq for Node<T> {
    fn eq(&self, other: &Self) -> bool {
        self.weight() == other.weight() && self.num() == other.num()
    }
}

impl<T: Debug> Eq for Node<T> {}

fn build_fieldop_hierarchy() -> Node<&'static FieldOpDescriptor> {
    let mut bh = BinaryHeap::with_capacity(FIELDOP_DESCRIPTORS.len());

    // valve's huffman-tree uses a variation which takes the node number into account
    let mut num = 0;

    for op in FIELDOP_DESCRIPTORS.iter() {
        bh.push(Node::Leaf {
            weight: op.weight,
            num,
            value: op,
        });
        num += 1;
    }

    while bh.len() > 1 {
        let left = bh.pop().unwrap();
        let right = bh.pop().unwrap();
        bh.push(Node::Branch {
            weight: left.weight() + right.weight(),
            num,
            left: Box::new(left),
            right: Box::new(right),
        });
        num += 1;
    }

    bh.pop().unwrap()
}

fn print_table(hierarchy: &Node<&'static FieldOpDescriptor>) {
    struct Leaf {
        op: &'static FieldOpDescriptor,
        id: usize,
        depth: usize,
    }
    let mut leafs: Vec<Leaf> = Vec::new();

    fn walk(
        hierarchy: &Node<&'static FieldOpDescriptor>,
        leafs: &mut Vec<Leaf>,
        id: usize,
        depth: usize,
    ) {
        match hierarchy {
            Node::Leaf { value: op, .. } => {
                leafs.push(Leaf { op, id, depth });
            }
            Node::Branch { left, right, .. } => {
                walk(right, leafs, (id << 1) | 1, depth + 1);
                walk(left, leafs, id << 1, depth + 1);
            }
        }
    }

    walk(hierarchy, &mut leafs, 0, 0);

    println!("{:>38} | weight |      id (op bits) | depth", "name");
    leafs.sort_by(|a, b| a.id.partial_cmp(&b.id).unwrap());
    leafs.iter().for_each(|leaf| {
        println!(
            "{:>38} | {:>6} | {:017b} | {:>5}",
            leaf.op.name, leaf.op.weight, leaf.id, leaf.depth
        );
    });
}

fn print_dot(hierarchy: &Node<&'static FieldOpDescriptor>) {
    fn walk(hierarchy: &Node<&'static FieldOpDescriptor>, id: usize, depth: usize) {
        match hierarchy {
            Node::Leaf { value, weight, .. } => {
                println!(
                    "  {} [label=\"{}\\nweight {}, id {}, depth {}\"];",
                    id, value.name, weight, id, depth
                );
            }
            Node::Branch { left, right, .. } => {
                println!("  {} [label=\"\"];", id);
                println!("  {} -> {};", id, (id << 1) | 1);
                println!("  {} -> {};", id, id << 1);

                walk(right, (id << 1) | 1, depth + 1);
                walk(left, id << 1, depth + 1);
            }
        }
    }

    println!("digraph Huffman {{");
    walk(hierarchy, 0, 0);
    println!("}}");
}

fn print_depth(hierarchy: &Node<&'static FieldOpDescriptor>) {
    fn walk(node: &Node<&'static FieldOpDescriptor>, depth: usize) -> usize {
        match node {
            Node::Leaf { .. } => depth,
            Node::Branch { left, right, .. } => {
                let left_depth = walk(left, depth + 1);
                let right_depth = walk(right, depth + 1);
                left_depth.max(right_depth)
            }
        }
    }
    let depth = walk(hierarchy, 0);
    println!("{}", depth);
}

// ----

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1);
    if cmd.is_none() {
        eprintln!("usage: huffmanfieldpath <table|dot|depth>");
        std::process::exit(42);
    }

    let hierarchy = build_fieldop_hierarchy();

    match cmd.unwrap().as_str() {
        // table command is useful for constructing id lookup "table" (manually;
        // see fieldpath.rs).
        "table" => print_table(&hierarchy),
        // dot command is fun xd. to get the visualisation run (graphviz
        // must be installed):
        // $ cargo run --bin huffmanfieldpath -- dot | dot -Tpng | feh -
        "dot" => print_dot(&hierarchy),
        // depth command finds the depth of the huffman.
        "depth" => print_depth(&hierarchy),
        cmd => eprintln!("invalid command: {}", cmd),
    }
}
