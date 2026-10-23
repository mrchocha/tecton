use std::io::{self, BufRead};

use crate::types::KeyValue;

#[derive(Debug)]
pub enum ByteReaderError {
    Io(io::Error),
    InvalidData,
    InvalidUtf8,
}

impl From<io::Error> for ByteReaderError {
    fn from(err: io::Error) -> Self {
        ByteReaderError::Io(err)
    }
}

pub struct ByteReader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> ByteReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    pub fn is_finished(&self) -> bool {
        self.position == self.bytes.len()
    }

    pub fn read_u32(&mut self) -> Result<u32, ByteReaderError> {
        let end = self
            .position
            .checked_add(4)
            .ok_or(ByteReaderError::InvalidData)?;

        let bytes = self
            .bytes
            .get(self.position..end)
            .ok_or(ByteReaderError::InvalidData)?;

        self.position = end;

        Ok(u32::from_be_bytes(
            bytes.try_into().map_err(|_| ByteReaderError::InvalidData)?,
        ))
    }

    pub fn read_u64(&mut self) -> Result<u64, ByteReaderError> {
        let end = self
            .position
            .checked_add(4)
            .ok_or(ByteReaderError::InvalidData)?;

        let bytes = self
            .bytes
            .get(self.position..end)
            .ok_or(ByteReaderError::InvalidData)?;

        self.position = end;

        Ok(u64::from_be_bytes(
            bytes.try_into().map_err(|_| ByteReaderError::InvalidData)?,
        ))
    }

    pub fn read_u8(&mut self) -> Result<u8, ByteReaderError> {
        let end = self
            .position
            .checked_add(1)
            .ok_or(ByteReaderError::InvalidData)?;

        let bytes = self
            .bytes
            .get(self.position..end)
            .ok_or(ByteReaderError::InvalidData)?;

        self.position = end;

        Ok(u8::from_be_bytes(
            bytes.try_into().map_err(|_| ByteReaderError::InvalidData)?,
        ))
    }

    pub fn read_usize(&mut self) -> Result<usize, ByteReaderError> {
        let end = self
            .position
            .checked_add(8)
            .ok_or(ByteReaderError::InvalidData)?;

        let bytes = self
            .bytes
            .get(self.position..end)
            .ok_or(ByteReaderError::InvalidData)?;

        self.position = end;

        Ok(usize::from_be_bytes(
            bytes.try_into().map_err(|_| ByteReaderError::InvalidData)?,
        ))
    }

    pub fn read_str(&mut self, len: usize) -> Result<String, ByteReaderError> {
        let end = self
            .position
            .checked_add(len)
            .ok_or(ByteReaderError::InvalidData)?;

        let bytes = self
            .bytes
            .get(self.position..end)
            .ok_or(ByteReaderError::InvalidData)?;

        self.position = end;

        String::from_utf8(bytes.to_vec()).map_err(|_| ByteReaderError::InvalidUtf8)
    }

    pub fn read_bytes_vec(&mut self, len: usize) -> Result<Vec<u8>, ByteReaderError> {
        let end = self
            .position
            .checked_add(len)
            .ok_or(ByteReaderError::InvalidData)?;

        let bytes = self
            .bytes
            .get(self.position..end)
            .ok_or(ByteReaderError::InvalidData)?;

        self.position = end;

        Ok(bytes.to_vec())
    }

    pub fn read_key_val(&mut self) -> Result<KeyValue, ByteReaderError> {
        let key_len = self.read_usize()?;
        let key = self.read_bytes_vec(key_len)?;

        let val_len = self.read_usize()?;
        let mut value: Option<Vec<u8>> = None;
        if val_len > 0 {
            value = Some(self.read_bytes_vec(val_len)?);
        }

        Ok(KeyValue { key, value })
    }
}

pub struct BufferByteReader<R> {
    reader: R,
}

impl<R: BufRead> BufferByteReader<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    pub fn read_u32(&mut self) -> Result<u32, ByteReaderError> {
        let mut buf = [0u8; 4];

        self.reader.read_exact(&mut buf)?;

        Ok(u32::from_be_bytes(buf))
    }

    pub fn read_u8(&mut self) -> Result<u8, ByteReaderError> {
        let mut buf = [0u8; 1];

        self.reader.read_exact(&mut buf)?;

        Ok(buf[0])
    }

    pub fn read_usize(&mut self) -> Result<usize, ByteReaderError> {
        let mut buf = [0u8; 8];

        self.reader.read_exact(&mut buf)?;

        Ok(usize::from_be_bytes(buf))
    }

    pub fn read_bytes_vec(&mut self, len: usize) -> Result<Vec<u8>, ByteReaderError> {
        let mut buf = vec![0u8; len];

        self.reader.read_exact(&mut buf)?;

        Ok(buf.to_vec())
    }

    pub fn read_key_val(&mut self) -> Result<KeyValue, ByteReaderError> {
        let key_len = self.read_usize()?;
        let key = self.read_bytes_vec(key_len)?;

        let val_len = self.read_usize()?;
        let mut value: Option<Vec<u8>> = None;
        if val_len > 0 {
            value = Some(self.read_bytes_vec(val_len)?);
        }

        Ok(KeyValue { key, value })
    }
}

pub struct FooterByteReader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> FooterByteReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            position: bytes.len(),
        }
    }

    pub fn read_u32(&mut self) -> Result<u32, ByteReaderError> {
        let start = self
            .position
            .checked_sub(4)
            .ok_or(ByteReaderError::InvalidData)?;

        let bytes = self
            .bytes
            .get(start..self.position)
            .ok_or(ByteReaderError::InvalidData)?;

        self.position = start;

        Ok(u32::from_be_bytes(
            bytes.try_into().map_err(|_| ByteReaderError::InvalidData)?,
        ))
    }

    pub fn read_bytes_vec(&mut self, len: usize) -> Result<Vec<u8>, ByteReaderError> {
        let start = self
            .position
            .checked_sub(len)
            .ok_or(ByteReaderError::InvalidData)?;

        let bytes = self
            .bytes
            .get(start..self.position)
            .ok_or(ByteReaderError::InvalidData)?;

        self.position = start;

        Ok(bytes.to_vec())
    }

    pub fn read_usize(&mut self) -> Result<usize, ByteReaderError> {
        let start = self
            .position
            .checked_sub(8)
            .ok_or(ByteReaderError::InvalidData)?;

        let bytes = self
            .bytes
            .get(start..self.position)
            .ok_or(ByteReaderError::InvalidData)?;

        self.position = start;

        Ok(usize::from_be_bytes(
            bytes.try_into().map_err(|_| ByteReaderError::InvalidData)?,
        ))
    }
}
