use std::{env, fs::File, io::Read};

fn main() {
    println!("On your marks...");
    let args: Vec<String> = env::args().collect();
    let filename = match args.iter().position(|x| x == "-i" || x == "--input") {
        Some(x) => &args[x + 1],
        None => {
            println!("Missing filename!");
            return;
        }
    };
    let compression = match args
        .iter()
        .any(|x| x == "-c" || x == "--compress" || x == "--deflate")
    {
        true => {
            if args
                .iter()
                .any(|y| y == "-d" || y == "--decompress" || y == "--inflate")
            {
                println!("Cannot compress and decompress at the same time!");
                return;
            } else {
                true
            }
        }
        false => {
            if args
                .iter()
                .any(|y| y == "-d" || y == "--decompress" || y == "--inflate")
            {
                false
            } else {
                println!("Missing de/compression flag!");
                return;
            }
        }
    };
    let mut rawdata = Vec::new();
    let _ = match File::open(filename) {
        Ok(mut f) => f.read_to_end(&mut rawdata),
        Err(e) => {
            println!("Error opening file: {}", e);
            return;
        }
    };
    let data = match compression {
        true => compress_data(&rawdata),
        false => decompress_data(&rawdata),
    };
    println!("Result: {:?}", data);
}

fn decompress_data(data: &[u8]) -> Vec<usize> {
    let mut finaldata = Vec::new();
    let mut blockbytes = 6u8;
    let mut blocklen = 0usize;
    let mut trimmer = 9u8;
    let mut tree = Tree::new();
    let mut byte = 0usize;
    let mut tmpval = Vec::new();
    let mut mode = 0u8;
    let mut maxsize = 8u8;
    loop {
        for bit in 0..maxsize {
            let bitvalue = (data[byte] >> (7 - bit)) & 1 == 1;
            match mode {
                0 => {
                    if blockbytes == 6u8 {
                        blockbytes = 5u8;
                        continue;
                    } else if blockbytes == 5u8 {
                        let bit1 = (data[byte] >> (8 - bit)) & 1;
                        blockbytes = ((bit1) << 1 | (bitvalue as u8)) + 1;
                        let mut result = vec![0u8; 4];
                        for i in 0..blockbytes {
                            let microdata = u16::from_le_bytes([data[byte], data[byte + 1]]);
                            result[i as usize] = (microdata << (bit + 1)).to_le_bytes()[0];
                            byte += 1;
                        }
                        blocklen = u32::from_le_bytes(result.try_into().unwrap()) as usize;
                        mode = 1;
                        continue;
                    }
                }
                1 => {
                    if trimmer == 9u8 {
                        trimmer = 8u8;
                        continue;
                    } else if trimmer == 8u8 {
                        let bit1 = (data[byte] >> (8 - bit)) & 1;
                        let bit3 = (data[byte] >> (6 - bit)) & 1;
                        trimmer = (bit1) << 2 | (bitvalue as u8) << 1 | (bit3);
                        continue;
                    } else {
                        print!("Constructing tree...");
                        mode = 2;
                        continue;
                    }
                }
                2 => {
                    construct_tree(&mut tree, bitvalue);
                    if check_tree(&tree, &mut |n: &NodeType| matches!(n, NodeType::Empty)) {
                        print!("\rConstructing tree... Done!\nPopulating tree...");
                        mode = 3;
                    }
                }
                3 => {
                    if tmpval.len() < blocklen {
                        tmpval.push(bitvalue);
                    }
                    if tmpval.len() == blocklen {
                        fill_tree(&mut tree, &tmpval);
                        tmpval = Vec::new();
                        if check_tree(&tree, &mut |n: &NodeType| matches!(n, NodeType::Data)) {
                            print!("\rPopulating tree... Done!\n");
                            mode = 4;
                        }
                    }
                }
                4 => {
                    tmpval.push(bitvalue);
                    match read_tree(&tree, &tmpval) {
                        Some(x) => {
                            finaldata.push(getval(&x));
                        }
                        _ => continue,
                    }
                    tmpval = Vec::new();
                }
                _ => {}
            }
        }
        if data.len() > byte + 1 {
            byte += 1;
            if data.len() == byte + 1 {
                maxsize = 8 - trimmer;
            }
        } else {
            break;
        }
    }
    finaldata
}

fn compress_data(data: &[u8]) -> Vec<usize> {
    println!("Compression not yet implemented!");
    let byte = 0usize;
    let mut mode = 0u8;
    let mut blockbytes = 6u8;
    loop {
        for bit in 0..8u8 {
            let bitvalue = (data[byte] >> (7 - bit)) & 1 == 1;
            match mode {
                0 => {
                    if blockbytes == 6u8 {
                        blockbytes = 5u8;
                        continue;
                    } else if blockbytes == 5u8 {
                        let bit1 = (data[byte] >> (8 - bit)) & 1;
                    }
                }
                1 => {
                    break;
                }
                _ => {}
            }
        }
    }
    Vec::new()
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

fn getval(data: &[bool]) -> usize {
    data.iter()
        .fold(0, |acc, &x| (acc << 1) | (if x { 1usize } else { 0usize }))
}

fn to_workable_bytes(data: &[usize]) -> Vec<u8> {
    let mut result = Vec::new();
    let mut remaining = [0; 8];
    let mut remaining_len = 0;

    for num in data {
        let num_bytes = num.to_be_bytes();
        if remaining_len > 0 {
            let mut combined = [0; 8];
            combined[..remaining_len].copy_from_slice(&remaining);
            combined[remaining_len..].copy_from_slice(&num_bytes[..8 - remaining_len]);
            result.push(combined);
            remaining_len = 0;
            remaining = [0; 8];
        }
        for chunk in num_bytes.chunks(8) {
            if chunk.len() == 8 {
                result.push(chunk.try_into().unwrap());
            } else {
                remaining.copy_from_slice(chunk);
                remaining_len = chunk.len();
            }
        }
    }
    if remaining_len > 0 {
        result.push(remaining);
    }
    result.concat()
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
