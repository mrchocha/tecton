use crate::MemTable;
use std::{cmp::Ordering, vec};

const MAX_HEIGHT: usize = 20;

pub struct SkipListNode {
    key: Vec<u8>,
    value: Option<Vec<u8>>,
    next: Vec<Option<usize>>,
}

pub struct SkipListMemTable {
    nodes: Vec<SkipListNode>,
    head: usize,
    current_height: usize,

    total_keys: usize,
    total_bytes_size: usize,
}

impl Default for SkipListMemTable {
    fn default() -> Self {
        Self::new()
    }
}

impl SkipListMemTable {
    pub fn new() -> Self {
        let mut nodes = Vec::new();
        nodes.push(SkipListNode {
            key: Vec::new(),
            value: None,
            next: vec![None; 1],
        });

        Self {
            nodes,
            head: 0,
            current_height: 1,
            total_keys: 0,
            total_bytes_size: 0,
        }
    }
}

impl SkipListMemTable {
    fn get_node_idx_by_key(&self, key: &[u8]) -> Option<usize> {
        let mut current_idx = self.head;

        for level in (0..self.current_height).rev() {
            loop {
                let next_idx = self.nodes[current_idx].next.get(level).copied().flatten();

                match next_idx {
                    Some(next_idx) => match self.nodes[next_idx].key.as_slice().cmp(key) {
                        Ordering::Less => {
                            current_idx = next_idx;
                        }

                        Ordering::Equal => {
                            return Some(next_idx);
                        }

                        Ordering::Greater => {
                            break;
                        }
                    },

                    None => {
                        break;
                    }
                }
            }
        }

        None
    }

    // find node_idxes <= the key
    fn get_nearest_node_idxes_by_key(&self, key: &[u8]) -> Vec<usize> {
        let mut update = vec![self.head; self.current_height];

        let mut current_idx = self.head;

        for level in (0..self.current_height).rev() {
            loop {
                let next_idx = self.nodes[current_idx].next.get(level).copied().flatten();

                match next_idx {
                    Some(next_idx) => match self.nodes[next_idx].key.as_slice().cmp(key) {
                        Ordering::Less => {
                            current_idx = next_idx;
                        }

                        Ordering::Equal | Ordering::Greater => {
                            break;
                        }
                    },

                    None => {
                        break;
                    }
                }
            }

            update[level] = current_idx;
        }

        update
    }

    fn random_height(&self) -> usize {
        let mut height = 1;

        while height < MAX_HEIGHT && rand::random::<bool>() {
            height += 1;
        }

        height
    }
}

impl MemTable for SkipListMemTable {
    fn put(&mut self, key: Vec<u8>, value: Option<Vec<u8>>) {
        // find and replace if existing node
        if let Some(node_idx) = self.get_node_idx_by_key(&key) {
            let node = &mut self.nodes[node_idx];

            let old_value_size = node.value.as_ref().map_or(0, Vec::len);

            node.value = value;

            self.total_bytes_size -= old_value_size;
            self.total_bytes_size += node.value.as_ref().unwrap().len();

            return;
        }
        let node_height = self.random_height();

        // Grow skip list if necessary
        if node_height > self.current_height {
            self.nodes[self.head].next.resize(node_height, None);

            self.current_height = node_height;
        }

        // find places to add node at each level
        let update = self.get_nearest_node_idxes_by_key(&key);
        let new_node_idx = self.nodes.len();

        let new_node = SkipListNode {
            key,
            value: value,
            next: vec![None; node_height],
        };

        self.nodes.push(new_node);

        for level in 0..node_height {
            let prev_idx = update[level];

            let next_idx = self.nodes[prev_idx].next[level];

            // prev -> new
            self.nodes[prev_idx].next[level] = Some(new_node_idx);

            // new -> next
            self.nodes[new_node_idx].next[level] = next_idx;
        }

        // note the size of key and value
        self.total_keys += 1;

        let node = &self.nodes[new_node_idx];

        self.total_bytes_size += node.key.len();

        if let Some(value) = &node.value {
            self.total_bytes_size += value.len();
        }
    }

    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        let node_idx = self.get_node_idx_by_key(key)?;
        let node = self.nodes.get(node_idx)?;

        node.value.clone()
    }

    fn delete(&mut self, key: &[u8]) {
        if let Some(node_idx) = self.get_node_idx_by_key(key)
            && let Some(node) = self.nodes.get_mut(node_idx)
        {
            self.total_bytes_size -= node.value.as_ref().map_or(0, Vec::len);
            node.value = None;
        }
    }

    fn size(&self) -> usize {
        self.total_keys
    }

    fn get_all_keys(&self) -> Vec<(Vec<u8>, Option<Vec<u8>>)> {
        let mut result = Vec::new();

        let mut current = self.nodes[self.head].next[0];

        while let Some(idx) = current {
            let node = &self.nodes[idx];

            result.push((node.key.clone(), node.value.clone()));

            current = node.next[0];
        }

        result
    }

    fn get_bytes_size(&self) -> usize {
        self.total_bytes_size
    }
}

#[cfg(test)]
mod tests {
    use std::assert_eq;

    use super::*;

    #[test]
    fn test_skip_list() {
        let mut skip_list = SkipListMemTable::new();

        let key1 = "rahul".as_bytes();
        let value1 = "chocha".as_bytes();

        skip_list.put(key1.to_vec(), Some(value1.to_vec()));

        assert_eq!(skip_list.get(key1), Some(value1.to_vec()));

        let key2 = "rahul1".as_bytes();
        let value2 = "chocha2".as_bytes();

        skip_list.put(key2.to_vec(), Some(value2.to_vec()));

        assert_eq!(skip_list.get(key2), Some(value2.to_vec()));

        let key1 = "rahul".as_bytes();
        let value1 = "chocha2".as_bytes();

        skip_list.put(key1.to_vec(), Some(value1.to_vec()));

        assert_eq!(skip_list.get(key1), Some(value1.to_vec()));

        assert_eq!(skip_list.get("rahul3".as_bytes()), None);
    }
}
