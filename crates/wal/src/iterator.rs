use std::{
    fs::File,
    io::{BufReader, Seek, SeekFrom},
};

use crate::{error::WalError, file::WalFile, manager::WalOptions, record::WalRecord, util};

pub struct WalIterator<'a> {
    pub seq_no: u32,

    file: WalFile,
    reader: BufReader<File>,

    options: &'a WalOptions,
}

impl<'a> WalIterator<'a> {
    const HEADER_SIZE: u64 = 4;

    pub fn new(options: &'a WalOptions) -> Result<Self, WalError> {
        let first_file = util::get_first_file(options, None)?.expect("Not able to find WAL files");

        let file_fd = first_file.get_file_fd()?;
        let mut reader = BufReader::new(file_fd);
        reader.seek(SeekFrom::Start(Self::HEADER_SIZE))?;

        Ok(Self {
            seq_no: first_file.seq_no,
            file: first_file,
            reader,
            options,
        })
    }

    pub fn has_next(&mut self) -> Result<bool, WalError> {
        let size = self.file.get_size();
        let offset = self.reader.stream_position()? as usize;

        if offset < size {
            return Ok(true);
        }

        if let Some(next_file) = util::get_first_file(self.options, Some(self.seq_no + 1))? {
            self.seq_no = self.file.seq_no;

            let file_fd = next_file.get_file_fd()?;
            let mut reader = BufReader::new(file_fd);
            reader.seek(SeekFrom::Start(Self::HEADER_SIZE))?;

            self.file = next_file;
            self.reader = reader;

            return Ok((self.reader.stream_position()? as usize) < self.file.get_size());
        }

        Ok(false)
    }

    pub fn next(&mut self) -> Result<Option<WalRecord>, WalError> {
        if !self.has_next()? {
            return Ok(None);
        }

        let wal_record = WalRecord::from_buffer(&mut self.reader)?;

        Ok(Some(wal_record))
    }
}
