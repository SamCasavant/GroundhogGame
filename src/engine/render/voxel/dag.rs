pub struct ChunkDag {
    pub root:    Node,
    pub ambient: u16,
}
impl ChunkDag {
    pub fn new(ambient: u16) -> ChunkDag {
        ChunkDag {
            root: Node::new(),
            ambient,
        }
    }
    pub fn get_pos(
        &self,
        // Enable parallelism by taking mut x
        mut x: i16,
        mut y: i16,
        mut z: i16,
    ) -> u16 {
        let mut node = &self.root;
        let mut range = i16::MAX;
        loop {
            range = range / 2;
            if range == 4 {
                return node.value;
            } else {
                let mut child_index = 0;
                if x > 0 {
                    child_index += 1;
                    x = x + range;
                } else {
                    x = x - range;
                }
                if y > 0 {
                    child_index += 2;
                    y = y + range;
                } else {
                    y = y - range;
                }
                if z > 0 {
                    child_index += 4;
                    z = z + range;
                } else {
                    z = z - range;
                }
                if let Some(child) =
                    node.children[child_index as usize].as_ref()
                {
                    node = child;
                } else {
                    return self.ambient;
                }
            }
        }
    }

    fn set_pos(
        &mut self,
        mut x: i16,
        mut y: i16,
        mut z: i16,
        value: u16,
    ) {
        let mut node = &mut self.root;
        let mut range = i16::MAX;
        loop {
            range = range / 2;
            if range == 4 {
                node.set_value(value);
                return;
            } else {
                let mut child_index = 0;
                if x > 0 {
                    child_index += 1;
                    x = x + range;
                } else {
                    x = x - range;
                }
                if y > 0 {
                    child_index += 2;
                    y = y + range;
                } else {
                    y = y - range;
                }
                if z > 0 {
                    child_index += 4;
                    z = z + range;
                } else {
                    z = z - range;
                }
                if let Some(child) = &node.children[child_index as usize] {
                    node = child;
                } else {
                    // TODO: Create branch to position
                }
            }
        }
    }
}

pub struct Node {
    // Child indices:
    // 0: x < 0, y < 0, z < 0
    // 1: x > 0, y < 0, z < 0
    // 2: x < 0, y > 0, z < 0
    // 3: x > 0, y > 0, z < 0
    // 4: x < 0, y < 0, z > 0
    // 5: x > 0, y < 0, z > 0
    // 6: x < 0, y > 0, z > 0
    // 7: x > 0, y > 0, z > 0
    pub children: Vec<Option<Node>>,
    pub value:    u16,
}
impl Node {
    pub fn new() -> Node {
        let mut children = Vec::new();
        children.resize_with(8, || None);
        Node { children, value: 0 }
    }
    pub fn add_child(
        &mut self,
        child: Node,
        position: usize,
    ) {
        if position >= 8 {
            panic!(
                "Attempted to add child to nonexistant index: {:?}",
                position
            )
        }
        self.children[position] = Some(child);
    }
    pub fn set_value(
        &mut self,
        value: u16,
    ) {
        self.value = value;
    }
}
