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
    println!("data: {:?}", data);
    let mut tree = LinkedList::new();
    // tree.head = Some(Box::new(Node {
    //     data: NodeData::Null,
    //     next: None,
    // }));
    for element in data.iter() {
        for bit in 0..8 {
            let bitvalue = (element >> (7 - bit)) & 1 == 1;
            //println!("{:?}", bitvalue);
            traverse_tree(&mut tree, bitvalue);
        }
    }
}

fn traverse_tree(tree: &mut LinkedList, bit: bool) {
    println!("tree: {:#?}", tree);
    match tree.head.take().unwrap().data {
        NodeData::List(mut list) => {
            traverse_tree(&mut list, bit);
        }
        NodeData::Null => {
            let new_node = match bit {
                true => Box::new(Node {
                    data: NodeData::List(Box::new(LinkedList {
                        head: Some(Box::new(Node {
                            data: tree.head.take().unwrap().data,
                            next: tree.head.take().unwrap().next,
                        })),
                    })),
                    next: Some(Box::new(Node {
                        data: NodeData::Int(0),
                        next: None,
                    })),
                }),
                _ => Box::new(Node {
                    data: NodeData::Int(0),
                    next: Some(tree.head.take().unwrap()),
                }),
            };
            tree.head = Some(new_node);
        }
        _ => {}
    }
}

#[derive(Debug)]
enum NodeData {
    Int(u32),
    List(Box<LinkedList>),
    Null,
}

impl NodeData {
    fn _is_null(&self) -> bool {
        matches!(self, NodeData::Null)
    }
}

#[derive(Debug)]
struct Node {
    data: NodeData,
    next: Option<Box<Node>>,
}

#[derive(Debug)]
struct LinkedList {
    head: Option<Box<Node>>,
}

impl LinkedList {
    fn new() -> LinkedList {
        //LinkedList { head: None }
        LinkedList {
            head: Some(Box::new(Node {
                data: NodeData::Null,
                next: None,
            })),
        }
    }
    fn _push(&mut self, data: NodeData) {
        let new_node = Box::new(Node {
            data,
            next: self.head.take(),
        });
        self.head = Some(new_node);
    }
    fn _pop(&mut self) -> Option<NodeData> {
        self.head.take().map(|node| {
            self.head = node.next;
            node.data
        })
    }
}
