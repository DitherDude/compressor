use std::{
    collections::HashMap,
    env,
    fs::{File, remove_file},
    io::{Read, Write},
};

fn main() {
    println!("On your marks...");
    let args: Vec<String> = env::args().collect();
    let mut force = false;
    let mut infilename = "";
    let mut outfilename = "";
    let mut blocksize = 0u32;
    let mut compression = false;
    let mut hashing = false;
    let mut zero = false;
    for (i, arg) in args.iter().enumerate() {
        if arg.starts_with("--") {
            match arg.strip_prefix("--").unwrap_or_default() {
                "blocksize" => blocksize = args[i + 1].parse().unwrap_or(0u32),
                "compress" | "deflate" => compression = true,
                "decompress" | "inflate" => compression = false,
                "force" => force = true,
                "input" => infilename = &args[i + 1],
                "output" => outfilename = &args[i + 1],
                "use-hashmap" => hashing = true,
                "zero" => zero = true,
                _ => {
                    println!("Expected long-name parameter.")
                }
            }
        } else if arg.starts_with("-") {
            let mut index = 1;
            for char in arg.strip_prefix("-").unwrap_or_default().chars() {
                match char {
                    'b' => {
                        blocksize = args[i + index].parse().unwrap_or(0u32);
                        index += 1;
                    }
                    'c' => compression = true,
                    'd' => compression = false,
                    'f' => {
                        force = true;
                    }
                    'i' => {
                        infilename = &args[i + index];
                        index += 1;
                    }
                    'o' => {
                        outfilename = &args[i + index];
                        index += 1;
                    }
                    'x' => hashing = true,
                    'z' => zero = true,
                    _ => {
                        println!("Expected short-name parameter or collection of.");
                        return;
                    }
                }
            }
        }
    }
    let mut rawdata = Vec::new();
    let _ = match File::open(infilename) {
        Ok(mut f) => f.read_to_end(&mut rawdata),
        Err(e) => {
            println!("Error opening file: {}", e);
            return;
        }
    };
    let mut file = if force {
        match File::create(outfilename) {
            Ok(file) => file,
            Err(e) => {
                println!("Error working with file: {}", e);
                return;
            }
        }
    } else {
        match File::create_new(outfilename) {
            Ok(file) => file,
            Err(e) => {
                println!("Error working with file: {}", e);
                return;
            }
        }
    };
    let mut data = match compression {
        true => compress_data(&rawdata, blocksize as usize, zero, hashing),
        false => decompress_data(&rawdata, hashing),
    };
    while data.len() % 8 != 0 {
        println!("Pop!");
        data.pop();
    }
    let data = data
        .chunks(8)
        .map(|chunk| {
            let mut array = [false; 8];
            for (i, &bit) in chunk.iter().enumerate() {
                array[i] = bit;
            }
            array
        })
        .map(|array| {
            array
                .into_iter()
                .fold(0u8, |acc, bit| (acc << 1) | (if bit { 1 } else { 0 }))
        })
        .collect::<Vec<u8>>();
    if !data.is_empty() {
        print!("Writing data...");
        let _ = std::io::stdout().flush();
        let _ = file.write_all(&data);
        let _ = file.flush();
        println!("\rWriting data... Done!");
    } else {
        match remove_file(outfilename) {
            Ok(_) => println!("Program failed! file {} was removed.", outfilename),
            Err(e) => println!(
                "Program failed! Additionally, program could not remove file {} due to {}.",
                outfilename, e
            ),
        }
    }
}

fn decompress_data(data: &[u8], hashing: bool) -> Vec<bool> {
    print!("Reading metadata...");
    let mut finaldata = Vec::<bool>::new();
    let mut datab: Vec<bool> = data
        .iter()
        .flat_map(|&byte| (0..8).rev().map(move |bit| (byte & (1 << bit)) != 0))
        .collect();
    let mut cursor = 0;
    let blockbytes: u8 = ((datab[cursor] as u8) << 1) | (datab[cursor + 1] as u8 + 1);
    cursor += 2;
    let mut result = Vec::new();
    for _ in 0..blockbytes {
        result.extend_from_slice(&datab[cursor..cursor + 8]);
        cursor += 8;
    }
    let blocklen = result.iter().fold(0, |acc, &x| acc * 2 + x as usize);
    let trimmer =
        ((datab[cursor] as u8) << 2) | ((datab[cursor + 1] as u8) << 1) | datab[cursor + 2] as u8;
    datab.drain(datab.len() - trimmer as usize..);
    let datablen = datab.len();
    cursor += 3;
    print!("\rReading metadata... Done!\nConstructing tree...");
    let _ = std::io::stdout().flush();
    let mut tree = Tree::new();
    while !check_tree(&tree, &mut |n: &NodeType| matches!(n, NodeType::Empty)) {
        construct_tree(&mut tree, datab[cursor]);
        cursor += 1;
    }
    print!("\rConstructing tree... Done!\nPopulating tree...");
    let _ = std::io::stdout().flush();
    let mut tmpval = Vec::new();
    while !check_tree(&tree, &mut |n: &NodeType| matches!(n, NodeType::Data)) {
        while tmpval.len() < blocklen {
            if cursor == datablen {
                break;
            }
            tmpval.push(datab[cursor]);
            cursor += 1;
        }
        fill_tree(&mut tree, &tmpval);
        tmpval = Vec::new();
    }
    tmpval = Vec::new();
    if hashing {
        print!("\rPopulating tree... Done!\nGenerating lookup table...");
        let _ = std::io::stdout().flush();
        let tree_lookup = lookup_tree(&tree, &[], false);
        print!("\rGenerating lookup table... Done!\nQuerying lookup table...");
        let _ = std::io::stdout().flush();
        while cursor < datablen {
            print!("\rQuerying lookup table... {}%", cursor * 100 / datablen);
            let _ = std::io::stdout().flush();
            tmpval.push(datab[cursor]);
            if let Some(x) = tree_lookup.get(&tmpval) {
                finaldata.extend_from_slice(x);
                tmpval = Vec::new();
            }
            cursor += 1;
        }
        println!("\rQuerying lookup table... Done!");
        let _ = std::io::stdout().flush();
    } else {
        print!("\rPopulating tree... Done!\nWalking paths...");
        let _ = std::io::stdout().flush();
        while cursor < datablen {
            print!("\rWalking paths... {}%", cursor * 100 / datablen);
            let _ = std::io::stdout().flush();
            tmpval.push(datab[cursor]);
            if let Some(x) = read_tree(&tree, &tmpval) {
                finaldata.extend_from_slice(&x);
                tmpval = Vec::new();
            }
            cursor += 1;
        }
        println!("\rWalking paths... Done!");
    }
    let _ = std::io::stdout().flush();
    finaldata
}

fn compress_data(data: &[u8], chunksize: usize, zerofill: bool, hashing: bool) -> Vec<bool> {
    print!("Configuring metadata...");
    let _ = std::io::stdout().flush();
    let blocked_blockbytes = chunksize.to_le_bytes();
    let bbl: u8;
    let blockbytes = if blocked_blockbytes[1] == 0 {
        bbl = 1;
        [false, false]
    } else if blocked_blockbytes[2] == 0 {
        bbl = 2;
        [false, true]
    } else if blocked_blockbytes[3] == 0 {
        bbl = 3;
        [true, false]
    } else {
        bbl = 4;
        [true, true]
    }
    .to_vec();
    let datab: Vec<bool> = data
        .iter()
        .flat_map(|&byte| (0..8).rev().map(move |bit| (byte & (1 << bit)) != 0))
        .collect();
    let datablen = datab.len();
    let mut cursor = 0;
    let mut bytes = Vec::new();
    for _ in 0..bbl {
        bytes.extend(
            (0..8)
                .map(|i| (chunksize >> (7 - i)) & 1 == 1)
                .collect::<Vec<bool>>(),
        );
    }
    print!("\rConfiguring metadata... Done!\nConstructing lookup table...");
    let _ = std::io::stdout().flush();
    let mut dictionary = HashMap::<NodeType, usize>::new();
    let mut block;
    while cursor < datablen {
        print!(
            "\rConstructing lookup table... {}%",
            cursor * 100 / datablen
        );
        let _ = std::io::stdout().flush();
        if cursor + chunksize > datablen {
            let spillover = (cursor + chunksize) - datablen;
            if zerofill {
                println!("\rPerforming zero fill. Expect volatile behaviour.");
                let _ = std::io::stdout().flush();
                block = [datab[cursor..].to_vec(), vec![false; chunksize - spillover]].concat();
                *dictionary.entry(NodeType::Value(block)).or_insert(0) += 1;
            } else {
                println!(
                    "\rData spillover of {} bits; data requires an additional {} bits to fill the required blocklen!",
                    spillover,
                    chunksize - spillover
                );
                return Vec::new();
            }
            break;
        }
        block = datab[cursor..cursor + chunksize].to_vec();
        *dictionary.entry(NodeType::Value(block)).or_insert(0) += 1;
        cursor += chunksize;
    }
    print!("\rConstructing lookup table... Done!\nSorting lookup table...");
    let _ = std::io::stdout().flush();
    let mut dictionary: Vec<(NodeType, usize)> =
        dictionary.iter().map(|(k, v)| (k.clone(), *v)).collect();
    dictionary.sort_by_key(|x| x.1);
    let mut tmpdictionary: Vec<(NodeType, usize)> = Vec::new();
    while dictionary.len() > 1 {
        for chunk in dictionary.chunks(2) {
            if chunk.len() == 2 {
                let node1 = chunk[0].0.clone();
                let node2 = chunk[1].0.clone();
                let value = chunk[0].1 + chunk[1].1;
                let key = Tree {
                    head: node1,
                    tail: node2,
                };
                tmpdictionary.push((NodeType::Tree(Box::new(key)), value));
            } else {
                tmpdictionary.push((chunk[0].0.clone(), chunk[0].1));
            }
        }
        dictionary = tmpdictionary;
        dictionary.sort_by_key(|x| x.1);
        tmpdictionary = Vec::new();
    }
    let tree = match &dictionary[0].0 {
        NodeType::Tree(subtree) => subtree,
        _ => &Tree {
            head: NodeType::Empty,
            tail: NodeType::Empty,
        },
    };
    let mut tree_paths: Vec<bool> = Vec::new();
    let mut chunk;
    cursor = 0;
    if hashing {
        print!("\rSorting lookup table... Done!\nGenerating lookup table...");
        let _ = std::io::stdout().flush();
        let tree_lookup = lookup_tree(tree, &[], true);
        print!("\rGenerating lookup table... Done!\nQuerying lookup table...");
        let _ = std::io::stdout().flush();
        while cursor < datablen {
            print!("\rQuerying lookup table... {}%", cursor * 100 / datablen);
            let _ = std::io::stdout().flush();
            if cursor + chunksize > datablen {
                let spillover = (cursor + chunksize) - datablen;
                if zerofill {
                    chunk = [datab[cursor..].to_vec(), vec![false; chunksize - spillover]].concat();
                    if let Some(x) = tree_lookup.get(&chunk) {
                        println!("{:?}", x);
                        tree_paths.extend(x.to_vec());
                    }
                }
                break;
            }
            chunk = datab[cursor..cursor + chunksize].to_vec();
            if let Some(x) = tree_lookup.get(&chunk) {
                tree_paths.extend(x.to_vec());
            }
            cursor += chunksize;
        }
        print!("\rQuerying lookup table... Done!\nCompiling data...");
    } else {
        print!("\rSorting lookup table... Done!\nWalking paths...");
        let _ = std::io::stdout().flush();
        while cursor < datablen {
            print!("\rWalking paths... {}%", cursor * 100 / datablen);
            let _ = std::io::stdout().flush();
            if cursor + chunksize > datablen {
                let spillover = (cursor + chunksize) - datablen;
                if zerofill {
                    chunk = [datab[cursor..].to_vec(), vec![false; chunksize - spillover]].concat();
                    if let Some(x) = _find_tree(tree, &chunk) {
                        println!("\rPath: {:?}", x);
                        tree_paths.extend(x);
                    }
                }
                break;
            }
            chunk = datab[cursor..cursor + chunksize].to_vec();
            if let Some(x) = _find_tree(tree, &chunk) {
                tree_paths.extend(x);
            }
            cursor += chunksize;
        }
        print!("\rWalking paths... Done!\nCompiling data...");
    }
    let _ = std::io::stdout().flush();
    let tree_construction = deconstruct_tree(tree);
    let tree_values = concat_tree(tree);
    let _ = std::io::stdout().flush();
    let mut remainder = 0;
    let mut finaldata = [
        blockbytes,
        bytes,
        [false; 3].to_vec(),
        tree_construction,
        tree_values,
        tree_paths,
    ]
    .concat();
    while (finaldata.len() % 8) != 0 {
        finaldata.push(false);
        remainder += 1;
    }
    let index = 2usize + (bbl as usize * 8);
    if remainder >= 4 {
        finaldata[index] = true;
        remainder -= 4;
    }
    if remainder >= 2 {
        finaldata[index + 1] = true;
        remainder -= 2;
    }
    if remainder >= 1 {
        finaldata[index + 2] = true;
    }
    println!("\rCompiling data... Done!");
    finaldata
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

fn deconstruct_tree(tree: &Tree) -> Vec<bool> {
    let head = match &tree.head {
        NodeType::Value(_) => vec![true],
        NodeType::Tree(subtree) => [vec![false], deconstruct_tree(subtree)].concat(),
        _ => Vec::new(),
    };
    let tail = match &tree.tail {
        NodeType::Value(_) => vec![false],
        NodeType::Tree(subtree) => [vec![true], deconstruct_tree(subtree)].concat(),
        _ => Vec::new(),
    };
    [head, tail].concat()
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

fn concat_tree(tree: &Tree) -> Vec<bool> {
    let head = match &tree.head {
        NodeType::Value(x) => x.to_vec(),
        NodeType::Tree(subtree) => concat_tree(subtree),
        _ => Vec::new(),
    };
    let tail = match &tree.tail {
        NodeType::Value(x) => x.to_vec(),
        NodeType::Tree(subtree) => concat_tree(subtree),
        _ => Vec::new(),
    };
    [head, tail].concat()
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

fn _find_tree(tree: &Tree, chunk: &[bool]) -> Option<Vec<bool>> {
    match &tree.head {
        NodeType::Value(x) => {
            if x == chunk {
                Some([false].to_vec())
            } else {
                None
            }
        }
        NodeType::Tree(subtree) => {
            let result = _find_tree(subtree, chunk);
            result.map(|x| [vec![false], x].concat())
        }
        _ => None,
    }
    .or_else(|| match &tree.tail {
        NodeType::Value(x) => {
            if x == chunk {
                Some([true].to_vec())
            } else {
                None
            }
        }
        NodeType::Tree(subtree) => {
            let result = _find_tree(subtree, chunk);
            result.map(|x| [vec![true], x].concat())
        }
        _ => None,
    })
}

fn lookup_tree(tree: &Tree, path: &[bool], compressing: bool) -> HashMap<Vec<bool>, Vec<bool>> {
    let mut result = HashMap::new();
    let subpath = [path, &[false]].concat();
    if let NodeType::Value(x) = &tree.head {
        if compressing {
            result.insert(x.to_vec(), subpath);
        } else {
            result.insert(subpath, x.to_vec());
        }
    } else if let NodeType::Tree(subtree) = &tree.head {
        result.extend(lookup_tree(subtree, &subpath, compressing));
    }
    let subpath = [path, &[true]].concat();
    if let NodeType::Value(x) = &tree.tail {
        if compressing {
            result.insert(x.to_vec(), subpath);
        } else {
            result.insert(subpath, x.to_vec());
        }
    } else if let NodeType::Tree(subtree) = &tree.tail {
        result.extend(lookup_tree(subtree, &subpath, compressing));
    }
    result
}

#[derive(Debug, Clone, Eq, Hash, PartialEq, PartialOrd, Ord)]
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

#[derive(Debug, Clone, Eq, Hash, PartialEq, PartialOrd, Ord)]
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
