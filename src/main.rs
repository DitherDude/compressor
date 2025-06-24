use std::{
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
        true => compress_data(&rawdata, blocksize, zero),
        false => decompress_data(&rawdata),
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
        let _ = file.write_all(&data);
        let _ = file.flush();
        println!("Done!");
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

fn decompress_data(data: &[u8]) -> Vec<bool> {
    let mut finaldata = Vec::<bool>::new();
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
                        let mut result = Vec::new();
                        for _ in 0..blockbytes {
                            for j in 2..8 {
                                result.push((data[byte] >> (7 - j)) & 1 == 1);
                            }
                            for j in 0..2 {
                                result.push((data[byte + 1] >> (7 - j)) & 1 == 1);
                            }
                            byte += 1;
                        }
                        blocklen = result.iter().fold(0, |acc, &x| acc * 2 + x as usize);
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
                        let bit2 = (data[byte] >> (7 - bit)) & 1;
                        let bit3 = (data[byte] >> (6 - bit)) & 1;
                        trimmer = (bit1) << 2 | (bit2) << 1 | (bit3);
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
                            continue;
                        }
                    }
                }
                4 => {
                    tmpval.push(bitvalue);
                    match read_tree(&tree, &tmpval) {
                        Some(x) => {
                            finaldata.extend_from_slice(&x);
                            tmpval = Vec::new();
                        }
                        _ => continue,
                    }
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

fn compress_data(data: &[u8], chunksize: u32, zerofill: bool) -> Vec<bool> {
    print!("Reading metadata...");
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
    let mut byte: usize;
    let mut bytes = Vec::new();
    for j in 0..bbl {
        byte = j as usize;
        bytes.push([false; 8]);
        for i in 0..8 {
            bytes[byte][i] = (chunksize >> (7 - i)) & 1 == 1;
        }
    }
    let mut dictionary = Vec::<Dictionary>::new();
    let mut block = Vec::<bool>::new();
    let mut byte = 0usize;
    print!("\rReading metadata... Done!\nConstructing lookup table... (0/2)");
    let _ = std::io::stdout().flush();
    loop {
        for bit in 0..8 {
            let bitvalue = (data[byte] >> (7 - bit)) & 1 == 1;
            block.push(bitvalue);
            if block.len() == chunksize as usize {
                match dictionary
                    .iter()
                    .position(|x| matches!(&x.key, NodeType::Value(v) if *v == block))
                {
                    Some(x) => dictionary[x].value += 1,
                    None => {
                        dictionary.push(Dictionary::newval(block, 1));
                    }
                }
                block = Vec::new();
            }
        }
        if data.len() > byte + 1 {
            byte += 1;
        } else {
            break;
        }
    }
    if block != Vec::new() {
        if zerofill {
            println!("\rPerforming zero fill. Expect volatile behaviour.");
            loop {
                block.push(false);
                if block.len() == chunksize as usize {
                    match dictionary
                        .iter()
                        .position(|x| matches!(&x.key, NodeType::Value(v) if *v == block))
                    {
                        Some(x) => dictionary[x].value += 1,
                        None => {
                            dictionary.push(Dictionary::newval(block, 1));
                        }
                    }
                    break;
                }
            }
        } else {
            println!(
                "\rData spillover of {} bits; data requires an additional {} bits to fill the required blocklen!",
                block.len(),
                chunksize - block.len() as u32
            );
            return Vec::new();
        }
    }
    print!("\rnConstructing lookup table... (1/2) ");
    let _ = std::io::stdout().flush();
    let mut tmpdictionary = Vec::new();
    while dictionary.len() > 1 {
        for chunk in dictionary.chunks(2) {
            if chunk.len() == 2 {
                let node1 = chunk[0].key.clone();
                let node2 = chunk[1].key.clone();
                let value = chunk[0].value + chunk[1].value;
                let key = Tree {
                    head: node1,
                    tail: node2,
                };
                tmpdictionary.push(Dictionary::newtree(key, value));
            } else {
                tmpdictionary.push(chunk[0].clone());
            }
        }
        dictionary = tmpdictionary;
        dictionary.sort_by_key(|x| x.value);
        tmpdictionary = Vec::new();
    }
    let tree = match &dictionary[0].key {
        NodeType::Tree(subtree) => subtree,
        _ => &Tree {
            head: NodeType::Empty,
            tail: NodeType::Empty,
        },
    };
    print!("\rConstructing lookup table... (2/2) \nWalking paths...");
    let _ = std::io::stdout().flush();
    let tree_construction = deconstruct_tree(tree);
    let tree_values = concat_tree(tree);
    let mut tree_paths: Vec<bool> = Vec::new();
    let mut chunk = Vec::new();
    for subbyte in data {
        for bit in 0..8 {
            let bitvalue = (subbyte >> (7 - bit)) & 1 == 1;
            chunk.push(bitvalue);
            if chunk.len() == chunksize as usize {
                let tree_path = find_tree(tree, &chunk);
                if let Some(x) = tree_path {
                    tree_paths.extend(x)
                };
                chunk = Vec::new();
            }
        }
    }
    if chunk != Vec::new() && zerofill {
        loop {
            chunk.push(false);
            if chunk.len() == chunksize as usize {
                let tree_path = find_tree(tree, &chunk);
                if let Some(x) = tree_path {
                    tree_paths.extend(x)
                };
                break;
            }
        }
    }
    let mut remainder = 0;
    let bytes = bytes.concat();
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

fn find_tree(tree: &Tree, chunk: &[bool]) -> Option<Vec<bool>> {
    match &tree.head {
        NodeType::Value(x) => {
            if x == chunk {
                Some([false].to_vec())
            } else {
                None
            }
        }
        NodeType::Tree(subtree) => {
            let result = find_tree(subtree, chunk);
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
            let result = find_tree(subtree, chunk);
            result.map(|x| [vec![true], x].concat())
        }
        _ => None,
    })
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

impl PartialEq for NodeType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (NodeType::Value(a), NodeType::Value(b)) => a == b,
            (NodeType::Data, NodeType::Data) => true,
            (NodeType::Tree(a), NodeType::Tree(b)) => a == b,
            (NodeType::Empty, NodeType::Empty) => true,
            _ => false,
        }
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

impl PartialEq for Tree {
    fn eq(&self, other: &Self) -> bool {
        self.head == other.head && self.tail == other.tail
    }
}

#[derive(Debug, Clone)]
struct Dictionary {
    key: NodeType,
    value: usize,
}

impl Dictionary {
    fn newval(key: Vec<bool>, value: usize) -> Dictionary {
        Dictionary {
            key: NodeType::Value(key),
            value,
        }
    }
    fn newtree(key: Tree, value: usize) -> Dictionary {
        Dictionary {
            key: NodeType::Tree(Box::new(key)),
            value,
        }
    }
}
