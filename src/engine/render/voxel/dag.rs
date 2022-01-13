pub struct ChunkDag {
    root: Children,
}

// Boxcar Children:
type Children = [Option<Box<Node>>; 8];

struct Node {
    // Child indices:
    // 0: -x, -y, -z
    // 1: +x, -y, -z
    // 2: -x, +y, -z
    // 3: +x, +y, -z
    // 4: -x, -y, +z
    // 5: +x, -y, +z
    // 6: -x, +y, +z
    // 7: +x, +y, +z
    val:      u16,
    children: Children,
}

impl Node {
    pub fn add_child(
        &mut self,
        val: u16,
        index: usize,
    ) -> Option<&mut Box<Node>> {
        let new_children: [Option<Box<Node>>; 8] = Default::default();
        let new_node = Box::new(Node {
            val,
            children: new_children,
        });
        self.children[index] = Some(new_node);
        self.children[index].as_mut()
    }
    pub fn get_child(
        &self,
        index: usize,
    ) -> Option<&Box<Node>> {
        self.children[index].as_ref()
    }
    pub fn get_child_mut(
        &mut self,
        index: usize,
    ) -> Option<&mut Box<Node>> {
        self.children[index].as_mut()
    }
    pub fn get_val_mut(
        &mut self,
        index: usize,
    ) -> Option<&mut u16> {
        self.children[index].as_mut().map(|node| &mut node.val)
    }
}
impl ChunkDag {
    pub fn new() -> Self {
        let root: [Option<Box<Node>>; 8] = Default::default();
        ChunkDag { root }
    }
    pub fn get_pos(
        &self,
        // Enable parallelism by taking mut x
        mut x: i16,
        mut y: i16,
        mut z: i16,
    ) -> u16 {
        let child_index =
            (x > 0) as usize + (y > 0) as usize * 2 + (z > 0) as usize * 4;

        let mut node = self.root[child_index].as_ref().unwrap();

        let mut range = i16::MAX / 2;
        x = if x > 0 { x - range } else { x + range };
        y = if y > 0 { y - range } else { y + range };
        z = if z > 0 { z - range } else { z + range };

        loop {
            let child_index = (x > 0) as usize
                + (y > 0) as usize * 2
                + (z > 0) as usize * 4;

            range /= 2;
            x = if x > 0 { x - range } else { x + range };
            y = if y > 0 { y - range } else { y + range };
            z = if z > 0 { z - range } else { z + range };

            if let Some(child) = node.get_child(child_index) {
                node = child;
            } else {
                return node.val;
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
        let child_index =
            (x > 0) as usize + (y > 0) as usize * 2 + (z > 0) as usize * 4;

        let mut node = self.root[child_index].as_mut().unwrap();

        let mut range = i16::MAX / 2;
        x = if x > 0 { x - range } else { x + range };
        y = if y > 0 { y - range } else { y + range };
        z = if z > 0 { z - range } else { z + range };

        loop {
            let child_index = (x > 0) as usize
                + (y > 0) as usize * 2
                + (z > 0) as usize * 4;

            range /= 2;
            x = if x > 0 { x - range } else { x + range };
            y = if y > 0 { y - range } else { y + range };
            z = if z > 0 { z - range } else { z + range };

            if node.get_child_mut(child_index).is_some() {
                let child = node.get_child_mut(child_index).unwrap();
                if range == 4 {
                    child.val = value;
                    return;
                } else {
                    node = child;
                }
            } else {
                node = node.add_child(value, child_index).unwrap()
            }
        }
    }
}
