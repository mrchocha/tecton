use std::fs;

use crate::{builder::SSTBuilder, file::SSTFile};

use mem_table::MemTable;

pub struct SSTOptions {
    pub file_path: String,
    pub max_file_size: usize,
}

pub struct SSTManager {
    options: SSTOptions,

    max_seq_no: u32,
}

impl SSTManager {
    pub fn new(options: SSTOptions) -> Self {
        Self {
            options,
            max_seq_no: 0,
        }
    }

    pub fn flush(
        &mut self,
        mem_table: Box<dyn MemTable>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.max_seq_no += 1;
        let sst_builder = SSTBuilder::new(mem_table, self.max_seq_no);

        sst_builder.flush(&self.options)?;

        Ok(())
    }

    pub fn list_files(options: &SSTOptions) -> Result<Vec<SSTFile>, Box<dyn std::error::Error>> {
        let mut wal_files = Vec::new();
        let path = options.file_path.clone();

        for entry in fs::read_dir(path)? {
            let path = entry?.path();

            if path.extension().is_none_or(|ext| ext != "sst") {
                continue;
            }

            let Some(path_str) = path.to_str() else {
                continue;
            };

            let wal_file = SSTFile::from_path(path_str)?;
            wal_files.push(wal_file);
        }

        Ok(wal_files)
    }
}

#[cfg(test)]
mod tests {

    use std::assert_eq;

    use {
        crate::manager::{SSTManager, SSTOptions},
        mem_table::{MemTable, btree_mem_table::BTreeMemTable},
    };

    #[test]
    fn create_sst() {
        let options = SSTOptions {
            file_path: "./test/sst".to_string(),
            max_file_size: 4096,
        };

        let sst_file_total = 10;

        let mut sst_manager = SSTManager::new(options);

        for sst_file in 0..sst_file_total {
            let mut mem_table: Box<dyn MemTable> = Box::new(BTreeMemTable::new());

            for id in 1..100 {
                let key = format!("key_{0}_{1}", sst_file, id).as_bytes().to_vec();
                let value = format!("value_{0}_{1}", sst_file, id).as_bytes().to_vec();

                mem_table.put(key, Some(value));
            }

            sst_manager
                .flush(mem_table)
                .expect("error while flushing table");
        }

        let options = SSTOptions {
            file_path: "./test/sst".to_string(),
            max_file_size: 4096,
        };

        let created_sst_files =
            SSTManager::list_files(&options).expect("error while reading sst files");

        assert_eq!(created_sst_files.len(), sst_file_total);
    }
}
