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
    let mut tree = TreeView::new();
    'outer: for byte in data.iter() {
        for bit in 0..8 {
            let bitvalue = (byte >> (7 - bit)) & 1 == 1;
            if !traverse_tree(&mut tree, bitvalue) {
                println!("EARLY TERMINATION!!!!!");
                break 'outer;
            }
        }
    }
    println!("tree: {:#?}", tree);
}

fn traverse_tree(tree: &mut TreeView, bit: bool) -> bool {
    if !match tree.data {
        NodeData::Empty => {
            tree.data = match bit {
                true => NodeData::Occupied,
                false => NodeData::Tree(Box::new(TreeView::new())),
            };
            true
        }
        NodeData::Tree(ref mut subtree) => traverse_tree(subtree, bit),
        _ => false,
    } && !match tree.next {
        NodeData::Empty => {
            tree.next = match bit {
                true => NodeData::Tree(Box::new(TreeView::new())),
                false => NodeData::Occupied,
            };
            true
        }
        NodeData::Tree(ref mut subtree) => traverse_tree(subtree, bit),
        _ => false,
    } {
        return false;
    }
    true
}

#[derive(Debug, Clone)]
enum NodeData {
    Value(u32),
    Occupied,
    Tree(Box<TreeView>),
    Empty,
}

impl NodeData {
    fn _is_null(&self) -> bool {
        matches!(self, NodeData::Empty)
    }
}

#[derive(Debug, Clone)]
struct TreeView {
    data: NodeData,
    next: NodeData,
}

impl TreeView {
    fn new() -> TreeView {
        TreeView {
            data: NodeData::Empty,
            next: NodeData::Empty,
        }
    }
    // fn _push(mut self, data: NodeData) {
    //     let new_node = Box::new(Node {
    //         data,
    //         next: NodeData::Tree(self),
    //     });
    //     self = LinkedList { head: new_node };
    // }
    // fn _pop(mut self) -> Option<NodeData> {
    //     self.map(|node| {
    //         self.head = node.next;
    //         node.data
    //     })
    // }
}
