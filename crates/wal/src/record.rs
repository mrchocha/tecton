use std::{fs::File, io::BufReader};

use common::{
    bytes_reader::{BufferByteReader, ByteReader},
    types::KeyValue,
};

use crate::error::WalError;

#[derive(Clone)]
pub enum OperationTypeEnum {
    INSERT = 0,
    UPDATE,
    DELETE,
}

impl OperationTypeEnum {
    fn from_u8(val: u8) -> Result<Self, WalError> {
        match val {
            0 => Ok(OperationTypeEnum::INSERT),
            1 => Ok(OperationTypeEnum::UPDATE),
            2 => Ok(OperationTypeEnum::DELETE),
            _ => Err(WalError::InvalidOperationType(val)),
        }
    }
}

pub struct WalRecord {
    pub term: u32,
    pub index: u32,
    pub op_type: OperationTypeEnum,
    pub key_val: KeyValue,
}

impl WalRecord {
    pub fn to_bytes(&self) -> Vec<u8> {
        let key_val_bytes = self.key_val.to_bytes();

        let size = 4 * 2 + 1 + key_val_bytes.len();
        let mut wal_record_bytes = Vec::with_capacity(size);
        wal_record_bytes.extend_from_slice(&self.term.to_be_bytes());
        wal_record_bytes.extend_from_slice(&self.index.to_be_bytes());
        wal_record_bytes.push(self.op_type.clone() as u8);
        wal_record_bytes.extend_from_slice(&key_val_bytes);

        let bytes_size: usize = 8 + 4 + size;
        let mut bytes = Vec::with_capacity(bytes_size);

        let crc32 = Self::checksum(&wal_record_bytes);

        bytes.extend_from_slice(&bytes_size.to_be_bytes());
        bytes.extend_from_slice(&crc32.to_be_bytes());
        bytes.extend_from_slice(&wal_record_bytes);

        bytes
    }

    pub fn checksum(bytes: &[u8]) -> u32 {
        let crc = crc::Crc::<u32>::new(&crc::CRC_32_ISCSI);
        let checksum: u32 = crc.checksum(bytes);

        checksum
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, WalError> {
        let mut breader = ByteReader::new(bytes);

        let _ = breader.read_usize()?;
        let crc32 = breader.read_u32()?;

        let term = breader.read_u32()?;
        let index = breader.read_u32()?;
        let op_type = OperationTypeEnum::from_u8(breader.read_u8()?)?;
        let key_val = breader.read_key_val()?;

        let wal_record = WalRecord {
            term,
            index,
            op_type,
            key_val,
        };

        if crc32 != Self::checksum(&wal_record.to_bytes()) {
            return Err(WalError::Message("Corrupted Wal record".to_string()));
        }

        Ok(wal_record)
    }

    pub fn from_buffer(buffer: &mut BufReader<File>) -> Result<Self, WalError> {
        let mut breader = BufferByteReader::new(buffer);

        let _ = breader.read_usize()?;
        let crc32 = breader.read_u32()?;

        let term = breader.read_u32()?;
        let index = breader.read_u32()?;
        let op_type = OperationTypeEnum::from_u8(breader.read_u8()?)?;
        let key_val = breader.read_key_val()?;

        let wal_record = WalRecord {
            term,
            index,
            op_type,
            key_val,
        };

        if crc32 != Self::checksum(&wal_record.to_bytes()) {
            return Err(WalError::Message("Corrupted Wal record".to_string()));
        }

        Ok(wal_record)
    }
}
