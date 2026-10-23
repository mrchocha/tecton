pub mod btree_mem_table;
pub mod skip_list_mem_table;

pub trait MemTable {
    fn put(&mut self, key: Vec<u8>, value: Option<Vec<u8>>);
    fn get(&self, key: &[u8]) -> Option<Vec<u8>>;
    fn delete(&mut self, key: &[u8]);
    fn size(&self) -> usize;
    fn get_all_keys(&self) -> Vec<(Vec<u8>, Option<Vec<u8>>)>;
    fn get_bytes_size(&self) -> usize;
}
