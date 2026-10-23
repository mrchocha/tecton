use std::{collections::BTreeMap, sync::RwLock};

use crate::MemTable;

pub struct BTreeMemTable {
    store: RwLock<BTreeMap<Vec<u8>, Option<Vec<u8>>>>,
    bytes_size: usize,
}

impl Default for BTreeMemTable {
    fn default() -> Self {
        Self::new()
    }
}

impl BTreeMemTable {
    pub fn new() -> Self {
        Self {
            store: RwLock::new(BTreeMap::new()),
            bytes_size: 0,
        }
    }
}

impl MemTable for BTreeMemTable {
    fn put(&mut self, key: Vec<u8>, value: Option<Vec<u8>>) {
        let mut store = self.store.write().unwrap();

        self.bytes_size += key.len() + value.iter().len();
        if let Some(prv_val) = store.insert(key, value)
            && let Some(non_empty_val) = prv_val
        {
            self.bytes_size -= non_empty_val.len()
        }
    }

    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        let store = self.store.read().unwrap();

        let value = store.get(key)?.as_ref()?;

        Some(value.clone())
    }

    fn delete(&mut self, key: &[u8]) {
        let mut store = self.store.write().unwrap();

        if let Some(prv_val) = store.insert(key.to_vec(), None)
            && let Some(non_empty_val) = prv_val
        {
            self.bytes_size -= non_empty_val.len()
        }
    }

    // fn scan(&self, start: String, end: String) -> Vec<KeyValue> {
    //     let store = self.store.read().unwrap();

    //     let start_bytes = start.into_bytes();
    //     let end_bytes = end.into_bytes();

    //     let mut kv_vect = Vec::new();

    //     for (key_bytes, opt_value_bytes) in store.range(start_bytes..=end_bytes) {
    //         if let Some(value_bytes) = opt_value_bytes {
    //             let key = String::from_utf8(key_bytes.clone()).expect("key decode error");
    //             let value = String::from_utf8(value_bytes.clone()).expect("value decode error");
    //             kv_vect.push(KeyValue { key, value });
    //         }
    //     }

    //     kv_vect
    // }

    fn size(&self) -> usize {
        let store = self.store.read().unwrap();

        store.len()
    }

    fn get_all_keys(&self) -> Vec<(Vec<u8>, Option<Vec<u8>>)> {
        let mut key_vals = Vec::new();

        let store = self.store.read().unwrap();

        for (key, val) in store.iter() {
            key_vals.push((key.clone(), val.clone()));
        }

        key_vals
    }

    fn get_bytes_size(&self) -> usize {
        self.bytes_size
    }
}

#[cfg(test)]
mod tests {
    use std::assert_eq;

    use super::*;

    #[test]
    fn test_btree() {
        let mut skip_list = BTreeMemTable::new();

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
