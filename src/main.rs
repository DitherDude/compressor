use std::{env, fs::File, io::Read};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Missing filename!");
        return;
    }
    let mut data = Vec::new();
    let _ = match File::open(&args[1]) {
        Ok(mut f) => f.read_to_end(&mut data),
        Err(e) => {
            println!("Error opening file: {}", e);
            return;
        }
    };
    let charlen = u32::from_le_bytes(data[0..4].try_into().unwrap());
    let data = &data[4..];
    println!("charlen: {}", charlen);
    let mut tree = Tree::new();
    let mut travel = Vec::new();
    'outer: for byte in data.iter() {
        for bit in 0..8 {
            let bitvalue = (byte >> (7 - bit)) & 1 == 1;
            if !construct_tree(&mut tree, bitvalue) {
                println!("Tree finished at {:?}", travel);
                break 'outer;
            }
            travel.push(if bitvalue { '1' } else { '0' });
        }
    }
    println!("tree: {:#?}", tree);
}

fn construct_tree(tree: &mut Tree, bit: bool) -> bool {
    if !match tree.head {
        NodeType::Empty => {
            tree.head = match bit {
                true => NodeType::Data,
                false => NodeType::Tree(Box::new(Tree::new())),
            };
            true
        }
        NodeType::Tree(ref mut subtree) => construct_tree(subtree, bit),
        _ => false,
    } && !match tree.tail {
        NodeType::Empty => {
            tree.tail = match bit {
                true => NodeType::Tree(Box::new(Tree::new())),
                false => NodeType::Data,
            };
            true
        }
        NodeType::Tree(ref mut subtree) => construct_tree(subtree, bit),
        _ => false,
    } {
        return false;
    }
    true
}

#[derive(Debug, Clone)]
enum NodeType {
    _Value(u32),
    Data,
    Tree(Box<Tree>),
    Empty,
}

impl NodeType {
    fn _is_null(&self) -> bool {
        matches!(self, NodeType::Empty)
    }
}

#[derive(Debug, Clone)]
struct Tree {
    head: NodeType,
    tail: NodeType,
}

impl Tree {
    fn new() -> Tree {
        Tree {
            head: NodeType::Empty,
            tail: NodeType::Empty,
        }
    }
}
