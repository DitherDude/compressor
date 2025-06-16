use std::{env, fs::File, io::Read};

fn main() {
    println!("On your marks...");
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
    let mut trimmer = 9u8;
    let data = &data[4..];
    let mut tree = Tree::new();
    let mut cursor = 0usize;
    let mut tmpval = Vec::new();
    let mut mode = 0u8;
    let mut maxsize = 8u8;
    loop {
        for bit in 0..maxsize {
            let bitvalue = (data[cursor] >> (7 - bit)) & 1 == 1;
            match mode {
                0 => {
                    if trimmer == 9u8 {
                        trimmer = 8u8;
                        continue;
                    }
                    if trimmer == 8u8 {
                        let bit1 = (data[cursor] >> (8 - bit)) & 1;
                        let bit3 = (data[cursor] >> (5 - bit)) & 1;
                        trimmer = (bit1) << 2 | (bitvalue as u8) << 1 | (bit3);
                        continue;
                    } else {
                        print!("Constructing tree...");
                        mode = 1;
                        continue;
                    }
                }
                1 => {
                    construct_tree(&mut tree, bitvalue);
                    if check_tree(&tree, &mut |n: &NodeType| matches!(n, NodeType::Empty)) {
                        print!("\rConstructing tree... Done!\nPopulating tree...");
                        mode = 2;
                    }
                }
                2 => {
                    if tmpval.len() < charlen as usize {
                        tmpval.push(bitvalue);
                    }
                    if tmpval.len() == charlen as usize {
                        fill_tree(&mut tree, &tmpval);
                        tmpval = Vec::new();
                        if check_tree(&tree, &mut |n: &NodeType| matches!(n, NodeType::Data)) {
                            print!("\rPopulating tree... Done!\n");
                            mode = 3;
                        }
                    }
                }
                3 => {
                    tmpval.push(bitvalue);
                    match read_tree(&tree, &tmpval) {
                        Some(x) => writeval(&x),
                        _ => continue,
                    }
                    tmpval = Vec::new();
                }
                _ => {}
            }
        }
        if data.len() > cursor + 1 {
            cursor += 1;
            if data.len() == cursor + 1 {
                maxsize = trimmer;
            }
        } else {
            println!();
            return;
        }
    }
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

fn fill_tree(tree: &mut Tree, data: &[bool]) -> bool {
    if !match tree.head {
        NodeType::Data => {
            tree.head = NodeType::Value(data.to_vec());
            true
        }
        NodeType::Tree(ref mut subtree) => fill_tree(subtree, data),
        _ => false,
    } && !match tree.tail {
        NodeType::Data => {
            tree.tail = NodeType::Value(data.to_vec());
            true
        }
        NodeType::Tree(ref mut subtree) => fill_tree(subtree, data),
        _ => false,
    } {
        return false;
    }
    true
}

fn check_tree(tree: &Tree, node: &mut impl FnMut(&NodeType) -> bool) -> bool {
    [&tree.head, &tree.tail].iter().all(|nodetype| {
        if node(nodetype) {
            false
        } else if let NodeType::Tree(subtree) = nodetype {
            check_tree(subtree, node)
        } else {
            true
        }
    })
}

fn read_tree(tree: &Tree, path: &[bool]) -> Option<Vec<bool>> {
    let node = if !path[0] { &tree.head } else { &tree.tail };
    match node {
        NodeType::Tree(subtree) => {
            if path.len() > 1 {
                read_tree(subtree, &path[1..])
            } else {
                None
            }
        }
        NodeType::Value(x) => Some(x.to_vec()),
        _ => None,
    }
}

fn writeval(data: &[bool]) {
    let char = data
        .iter()
        .fold(0, |acc, &x| (acc << 1) | (if x { 1u8 } else { 0u8 }));
    print!("{}", String::from_utf8_lossy(&[char]));
}

#[derive(Debug, Clone)]
enum NodeType {
    Value(Vec<bool>),
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
