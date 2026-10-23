use std::collections::VecDeque;

use crate::error::StoreError;

use {
    common::types::KeyValue,
    mem_table::{MemTable, btree_mem_table::BTreeMemTable},
    wal::{
        manager::{WalManager, WalOptions},
        record::{OperationTypeEnum, WalRecord},
    },
};

use sst::manager::{SSTManager, SSTOptions};

pub struct StorageOptions {
    /* WAL file options */
    pub wal_file_path: String,
    pub wal_max_file_size: usize,

    /* SST file options */
    pub sst_file_path: String,
    pub sst_max_file_size: usize,
}

impl StorageOptions {
    pub fn new() -> Self {
        Self {
            wal_file_path: "./data/wal".to_string(),
            wal_max_file_size: 4069,
            sst_file_path: "./data/sst".to_string(),
            sst_max_file_size: 4069,
        }
    }

    pub fn with_wal_file_path(&mut self, wal_file_path: String) -> &Self {
        self.wal_file_path = wal_file_path;
        self
    }

    pub fn with_wal_max_file_size(&mut self, wal_max_file_size: usize) -> &Self {
        self.wal_max_file_size = wal_max_file_size;
        self
    }

    pub fn with_sst_file_path(&mut self, sst_file_path: String) -> &Self {
        self.sst_file_path = sst_file_path;
        self
    }

    pub fn with_sst_max_file_size(&mut self, sst_max_file_size: usize) -> &Self {
        self.sst_max_file_size = sst_max_file_size;
        self
    }

    fn to_wal_options(&self) -> WalOptions {
        WalOptions {
            file_path: self.wal_file_path.clone(),
            max_file_size: self.wal_max_file_size,
        }
    }

    fn to_sst_options(&self) -> SSTOptions {
        SSTOptions {
            file_path: self.sst_file_path.clone(),
            max_file_size: self.sst_max_file_size,
        }
    }
}

pub struct Storage {
    options: StorageOptions,

    term: u32,
    index: u32,

    wal_manager: WalManager,
    sst_manager: SSTManager,

    mem_table: Box<dyn MemTable>,

    immutable_mem_tables: VecDeque<Box<dyn MemTable>>,
}

impl Storage {
    pub fn new(options: StorageOptions) -> Result<Self, StoreError> {
        let wal_manager = WalManager::new(options.to_wal_options())?;
        let sst_manager = SSTManager::new(options.to_sst_options());

        let mem_table = Box::new(BTreeMemTable::new());

        Ok(Self {
            options,
            term: 0,
            index: 0,
            wal_manager,
            sst_manager,
            mem_table,
            immutable_mem_tables: VecDeque::new(),
        })
    }

    pub fn put(&mut self, key: String, value: String) -> Result<(), StoreError> {
        let current_index = self.index;
        self.index += 1;

        self.wal_manager.write(&WalRecord {
            term: self.term,
            index: current_index,
            op_type: OperationTypeEnum::INSERT,
            key_val: KeyValue {
                key: key.clone().into_bytes(),
                value: Some(value.clone().into_bytes()),
            },
        })?;

        if self.mem_table.get_bytes_size() >= self.options.sst_max_file_size {
            self.flush()?;
        }

        self.mem_table
            .put(key.into_bytes(), Some(value.into_bytes()));

        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<String> {
        let val = self.mem_table.get(key.as_bytes())?;
        String::from_utf8(val).ok()
    }

    pub async fn delete(&mut self, key: String) -> Result<(), StoreError> {
        let current_index = self.index;
        self.index += 1;

        self.wal_manager.write(&WalRecord {
            term: self.term,
            index: current_index,
            op_type: OperationTypeEnum::DELETE,
            key_val: KeyValue {
                key: key.clone().into_bytes(),
                value: None,
            },
        })?;

        self.mem_table.put(key.into_bytes(), None);

        Ok(())
    }

    pub fn reset_mem_table(&mut self) {
        let old_sst = std::mem::replace(&mut self.mem_table, Box::new(BTreeMemTable::new()));
        self.immutable_mem_tables.push_front(old_sst);
    }

    pub fn flush(&mut self) -> Result<(), StoreError> {
        while self.immutable_mem_tables.len() > 0 {
            if let Some(sst) = self.immutable_mem_tables.pop_back() {
                self.sst_manager.flush(sst)?;
            }
        }
        Ok(())
    }
}
