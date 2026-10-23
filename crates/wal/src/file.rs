use std::fs::File;
use std::fs::OpenOptions;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use crate::error::WalError;
use crate::manager::WalOptions;
use crate::record::WalRecord;

pub struct WalFile {
    pub seq_no: u32,
    pub name: String,
    pub path: PathBuf,

    file_writer: BufWriter<File>,
    size: usize,
}

impl WalFile {
    pub fn create(options: &WalOptions, seq_no: u32) -> Result<Self, WalError> {
        let name: String = format!("{:020}.wal", seq_no);
        let path = PathBuf::from(options.file_path.clone()).join(&name);

        let file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(&path)?;

        let mut writer = BufWriter::new(file);

        writer.write_all(&seq_no.to_be_bytes())?;
        writer.flush()?;

        Ok(Self {
            name,
            path,
            seq_no,
            file_writer: writer,
            size: 4, // with seq no
                     // buffer: Vec::new(),
        })
    }

    pub fn from_path(str_path: &str) -> Result<Self, WalError> {
        let path = Path::new(str_path);

        let file = OpenOptions::new().read(true).append(true).open(path)?;

        let reader_file = file.try_clone()?;
        let mut reader = BufReader::new(reader_file);

        let writer = BufWriter::new(file);

        let metadata = writer.get_ref().metadata()?;
        let file_size = metadata.len() as usize;

        let mut buffer = [0u8; 4];
        reader.read_exact(&mut buffer)?;

        let seq_no = u32::from_be_bytes(buffer);

        let name = path
            .file_name()
            .ok_or("path has no filename")?
            .to_str()
            .ok_or("filename is not valid UTF-8")?
            .to_string();

        Ok(WalFile {
            name,
            path: path.to_path_buf(),
            seq_no,
            file_writer: writer,
            // buffer: Vec::new(),
            size: file_size,
        })
    }

    pub fn get_size(&self) -> usize {
        self.size
    }

    pub fn get_file_fd(&self) -> Result<File, WalError> {
        Ok(self.file_writer.get_ref().try_clone()?)
    }

    pub fn append(&mut self, wal_record: &WalRecord) -> Result<(), WalError> {
        let bytes = wal_record.to_bytes();
        self.file_writer.write_all(&bytes)?;
        self.size += bytes.len();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_create() {}
}
