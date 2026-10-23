use xxhash_rust::xxh64::xxh64;

use common::bytes_reader::{ByteReader, ByteReaderError};
pub struct BloomFilter {
    bit_size: u64,
    num_hash: u64,
    bit_arr: Vec<u8>,
}

impl BloomFilter {
    pub fn new(num_elems: u64, acceptable_fp: f64) -> Self {
        let ln2 = 2.0_f64.ln();
        let bit_size = (-(num_elems as f64 * acceptable_fp.ln()) / ln2.powi(2)).ceil() as u64;

        let num_hash = ((bit_size as f64 / num_elems as f64) * ln2).ceil() as u64;

        let bit_arr = vec![0u8; (bit_size as usize).div_ceil(8)];

        BloomFilter {
            bit_arr,
            bit_size,
            num_hash,
        }
    }

    fn hashes(&self, key: &[u8]) -> Vec<u64> {
        let mut indices = Vec::new();

        let hash_1 = xxh64(key, 0);
        let hash_2 = xxh64(key, 10);

        for i in 0..self.num_hash {
            let position = ((hash_1 % self.bit_size)
                + ((i % self.bit_size) * (hash_2 % self.bit_size)) % self.bit_size)
                % self.bit_size;
            indices.push(position);
        }

        indices
    }

    pub fn add(&mut self, key: &[u8]) {
        for index in self.hashes(key) {
            let byte_index = (index / 8) as usize;
            let bit_offset = index % 8;

            self.bit_arr[byte_index] |= 1 << bit_offset;
        }
    }

    pub fn is_present(&self, key: &[u8]) -> bool {
        for index in self.hashes(key) {
            let byte_index = (index / 8) as usize;
            let bit_offset = index % 8;

            if let Some(data) = self.bit_arr.get(byte_index)
                && *data & (1 << bit_offset) == 0
            {
                return false;
            }
        }
        true
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let total_size = 8 + 8 + self.bit_arr.len();
        let mut binary = Vec::with_capacity(total_size);
        binary.extend_from_slice(&self.num_hash.to_be_bytes());
        binary.extend_from_slice(&self.bit_size.to_be_bytes());
        binary.extend_from_slice(&self.bit_arr);
        binary
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ByteReaderError> {
        let mut btreader = ByteReader::new(bytes);
        let num_hash = btreader.read_u64()?;
        let bit_size = btreader.read_u64()?;
        let bit_arr = btreader.read_bytes_vec(bit_size as usize)?;

        Ok(Self {
            bit_size,
            num_hash,
            bit_arr,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{assert_eq, format};

    use super::*;

    #[test]
    fn test_bloom() {
        let mut b_filter = BloomFilter::new(10, 0.001);

        for i in 0..10 {
            let key = format!("key_{0}", i);
            b_filter.add(key.as_bytes());
        }

        for i in 0..10 {
            let key = format!("key_{0}", i);
            assert_eq!(b_filter.is_present(key.as_bytes()), true);
        }

        for i in 11..20 {
            let key = format!("key_{0}", i);
            assert_eq!(b_filter.is_present(key.as_bytes()), false);
        }
    }
}
