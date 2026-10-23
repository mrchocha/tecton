use crate::{error::WalError, file::WalFile, record::WalRecord, util};

pub struct WalOptions {
    pub file_path: String,
    pub max_file_size: usize,
}

pub struct WalManager {
    options: WalOptions,

    current_file: WalFile,

    // reusable_files: Vec<WalFile>,
    max_wal_file_seq_no: u32,
}

impl WalManager {
    pub fn new(options: WalOptions) -> Result<WalManager, WalError> {
        let mut file = util::get_first_file(&options, Some(0))?;
        if file.is_none() {
            file = Some(WalFile::create(&options, 1)?);
        }

        let wal_file = file.expect("Wal file not found");

        Ok(WalManager {
            options,
            max_wal_file_seq_no: wal_file.seq_no,
            current_file: wal_file,
            // reusable_files: Vec::new(),
        })
    }

    fn should_rotate(&self) -> bool {
        self.current_file.get_size() >= self.options.max_file_size
    }

    fn rotate(&mut self) -> Result<(), WalError> {
        self.max_wal_file_seq_no += 1;
        let file = WalFile::create(&self.options, self.max_wal_file_seq_no)?;

        self.current_file = file;

        Ok(())
    }

    pub fn write(&mut self, wal_record: &WalRecord) -> Result<(), WalError> {
        if self.should_rotate() {
            self.rotate()?
        }

        self.current_file.append(wal_record)?;

        Ok(())
    }
}
