use quote::{__private::TokenStream, format_ident, quote};
use std::{cmp::Reverse, collections::BinaryHeap};

struct FieldOp {
    name: &'static str,
    weight: u32,
}

const FIELD_OPS: [FieldOp; 40] = [
    FieldOp {
        name: "PlusOne",
        weight: 36271,
    },
    FieldOp {
        name: "PlusTwo",
        weight: 10334,
    },
    FieldOp {
        name: "PlusThree",
        weight: 1375,
    },
    FieldOp {
        name: "PlusFour",
        weight: 646,
    },
    FieldOp {
        name: "PlusN",
        weight: 4128,
    },
    FieldOp {
        name: "PushOneLeftDeltaZeroRightZero",
        weight: 35,
    },
    FieldOp {
        name: "PushOneLeftDeltaZeroRightNonZero",
        weight: 3,
    },
    FieldOp {
        name: "PushOneLeftDeltaOneRightZero",
        weight: 521,
    },
    FieldOp {
        name: "PushOneLeftDeltaOneRightNonZero",
        weight: 2942,
    },
    FieldOp {
        name: "PushOneLeftDeltaNRightZero",
        weight: 560,
    },
    FieldOp {
        name: "PushOneLeftDeltaNRightNonZero",
        weight: 471,
    },
    FieldOp {
        name: "PushOneLeftDeltaNRightNonZeroPack6Bits",
        weight: 10530,
    },
    FieldOp {
        name: "PushOneLeftDeltaNRightNonZeroPack8Bits",
        weight: 251,
    },
    FieldOp {
        name: "PushTwoLeftDeltaZero",
        weight: 1,
    },
    FieldOp {
        name: "PushTwoPack5LeftDeltaZero",
        weight: 1,
    },
    FieldOp {
        name: "PushThreeLeftDeltaZero",
        weight: 1,
    },
    FieldOp {
        name: "PushThreePack5LeftDeltaZero",
        weight: 1,
    },
    FieldOp {
        name: "PushTwoLeftDeltaOne",
        weight: 1,
    },
    FieldOp {
        name: "PushTwoPack5LeftDeltaOne",
        weight: 1,
    },
    FieldOp {
        name: "PushThreeLeftDeltaOne",
        weight: 1,
    },
    FieldOp {
        name: "PushThreePack5LeftDeltaOne",
        weight: 1,
    },
    FieldOp {
        name: "PushTwoLeftDeltaN",
        weight: 1,
    },
    FieldOp {
        name: "PushTwoPack5LeftDeltaN",
        weight: 1,
    },
    FieldOp {
        name: "PushThreeLeftDeltaN",
        weight: 1,
    },
    FieldOp {
        name: "PushThreePack5LeftDeltaN",
        weight: 1,
    },
    FieldOp {
        name: "PushN",
        weight: 1,
    },
    FieldOp {
        name: "PushNAndNonTopological",
        weight: 310,
    },
    FieldOp {
        name: "PopOnePlusOne",
        weight: 2,
    },
    FieldOp {
        name: "PopOnePlusN",
        weight: 1,
    },
    FieldOp {
        name: "PopAllButOnePlusOne",
        weight: 1837,
    },
    FieldOp {
        name: "PopAllButOnePlusN",
        weight: 149,
    },
    FieldOp {
        name: "PopAllButOnePlusNPack3Bits",
        weight: 300,
    },
    FieldOp {
        name: "PopAllButOnePlusNPack6Bits",
        weight: 634,
    },
    FieldOp {
        name: "PopNPlusOne",
        weight: 1,
    },
    FieldOp {
        name: "PopNPlusN",
        weight: 1,
    },
    FieldOp {
        name: "PopNAndNonTopographical",
        weight: 1,
    },
    FieldOp {
        name: "NonTopoComplex",
        weight: 76,
    },
    FieldOp {
        name: "NonTopoPenultimatePlusOne",
        weight: 271,
    },
    FieldOp {
        name: "NonTopoComplexPack4Bits",
        weight: 99,
    },
    FieldOp {
        name: "FieldPathEncodeFinish",
        weight: 25474,
    },
];

// based on https://github.com/Lakret/huffman-rs/blob/4e2f759e2ca384108e5c95bc9cd365fad1d48364/src/huffman.rs
// NOTE: Tree does not have an A: Allocator generic param because it's not
// supposed to be used in real code, but for code generation.
#[derive(Debug, PartialEq, Eq)]
enum Tree<V> {
    Leaf {
        weight: u32,
        value: V,
    },
    Node {
        weight: u32,
        left: Box<Tree<V>>,
        right: Box<Tree<V>>,
    },
}

impl<V: Clone> Tree<V> {
    fn weight(&self) -> u32 {
        match self {
            Self::Leaf { weight, .. } => *weight,
            Self::Node { weight, .. } => *weight,
        }
    }
}

impl<V: Clone + Eq> Ord for Tree<V> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.weight().cmp(&other.weight())
    }
}

impl<V: Clone + Eq> PartialOrd for Tree<V> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn build_tree() -> Tree<usize> {
    let mut heap = BinaryHeap::with_capacity(FIELD_OPS.len());
    for (i, fop) in FIELD_OPS.iter().enumerate() {
        heap.push(Reverse(Tree::Leaf {
            weight: fop.weight,
            value: i,
        }));
    }

    while heap.len() > 1 {
        let left = heap.pop().unwrap().0;
        let right = heap.pop().unwrap().0;
        let merged_node = Tree::Node {
            weight: left.weight() + right.weight(),
            left: Box::new(left),
            right: Box::new(right),
        };
        heap.push(Reverse(merged_node));
    }

    heap.pop().unwrap().0
}

fn walk_tree(tree: &Tree<usize>, id: u32, match_arms: &mut Vec<TokenStream>) {
    match tree {
        Tree::Leaf { value, .. } => {
            let field_op_name = format_ident!("{}", FIELD_OPS[*value].name);
            match_arms.push(quote! { #id => Some(#field_op_name), });
        }
        Tree::Node { left, right, .. } => {
            walk_tree(&left, id << 1 | 0, match_arms);
            walk_tree(&right, id << 1 | 1, match_arms);
        }
    }
}

pub fn build_fops() -> String {
    let fops =
        syn::parse_file(include_str!("fops.inline")).expect("valid rust code in fops.inline file");
    let tree = build_tree();
    let mut match_arms = vec![];
    walk_tree(&tree, 0, &mut match_arms);
    let ts = quote! {
        #fops
        pub type FieldOp = fn(fp: &mut FieldPath, br: &mut BitReader) -> Result<()>;
        #[inline(always)]
        pub fn lookup(id: u32) -> Option<FieldOp> {
            match id {
                #(#match_arms)*
                _ => None,
            }
        }
    };
    prettyplease::unparse(&syn::parse_quote! { #ts })
}

#[cfg(test)]
mod tests {
    use super::build_fops;

    #[test]
    fn test_build_field_ops_tree() {
        println!("{}", build_fops());
    }
}
