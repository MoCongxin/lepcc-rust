// Copyright 2016 Esri
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! FlagBytes compression implementation

use crate::bit_stuffer2::BitStuffer2;
use crate::huffman::Huffman;
use crate::common::compute_checksum_fletcher32;
use crate::error::{LepccError, Result};
use crate::types::Byte;
use std::io::{Cursor, Write};

const FILE_KEY: &[u8; 10] = b"FlagBytes ";
const K_CURR_VERSION: u16 = 1;
const HEADER_SIZE: usize = 32; // TopHeader(16) + Header1(16)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompressionMethod {
    BitStuff = 0,
    HuffmanCodec = 1,
}

impl TryFrom<u8> for CompressionMethod {
    type Error = LepccError;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(CompressionMethod::BitStuff),
            1 => Ok(CompressionMethod::HuffmanCodec),
            _ => Err(LepccError::WrongParam(format!("Invalid compression method: {}", value))),
        }
    }
}

pub struct FlagBytesEncoder {
    num_bytes_needed: i64,
    min_value: Byte,
    compression_method: CompressionMethod,
    huffman: Huffman,
    #[allow(dead_code)]
    data_vec: Vec<u32>,
    #[allow(dead_code)]
    byte_vec: Vec<Byte>,
}


impl Default for FlagBytesEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl FlagBytesEncoder {
    pub fn new() -> Self {
        FlagBytesEncoder {
            num_bytes_needed: 0,
            min_value: 0,
            compression_method: CompressionMethod::BitStuff,
            huffman: Huffman::new(),
            data_vec: Vec::new(),
            byte_vec: Vec::new(),
        }
    }

    /// Compute the number of bytes needed to encode flag bytes
    pub fn compute_num_bytes_needed(&mut self, flag_bytes: &[Byte]) -> Result<i64> {
        if flag_bytes.is_empty() {
            return Err(LepccError::WrongParam("No flag bytes provided".to_string()));
        }

        // Calculate histogram
        let mut histo = vec![0i32; 256];
        let mut num_non_zero_bins = 0;

        for &byte in flag_bytes {
            if histo[byte as usize] == 0 {
                num_non_zero_bins += 1;
            }
            histo[byte as usize] += 1;
        }

        // Try Huffman
        let mut n_bytes_huffman: Option<i64> = None;
        if num_non_zero_bins > 1 {
            self.compression_method = CompressionMethod::HuffmanCodec;
            self.min_value = 0;
            n_bytes_huffman = self.huffman.compute_num_bytes_needed_to_encode(&histo);
        }

        // Bit stuff - find range
        let mut i0 = 0;
        while histo[i0] == 0 && i0 < 255 {
            i0 += 1;
        }

        let mut i1 = 255;
        while histo[i1] == 0 && i1 > 0 {
            i1 -= 1;
        }

        let max_elem = (i1 - i0) as u8;
        let _bit_stuffer = BitStuffer2;
        let n_bytes_bit_stuff = BitStuffer2::compute_num_bytes_needed_simple(flag_bytes.len() as u32, max_elem as u32) as i64;

        // Choose the better method
        if let Some(n_bytes_huff) = n_bytes_huffman {
            if n_bytes_huff > 0 && n_bytes_huff < n_bytes_bit_stuff {
                self.compression_method = CompressionMethod::HuffmanCodec;
                self.num_bytes_needed = HEADER_SIZE as i64 + n_bytes_huff;
            } else {
                self.compression_method = CompressionMethod::BitStuff;
                self.min_value = i0 as u8;
                self.num_bytes_needed = HEADER_SIZE as i64 + n_bytes_bit_stuff;
            }
        } else {
            self.compression_method = CompressionMethod::BitStuff;
            self.min_value = i0 as u8;
            self.num_bytes_needed = HEADER_SIZE as i64 + n_bytes_bit_stuff;
        }

        Ok(self.num_bytes_needed)
    }

    /// Encode flag bytes
    pub fn encode(&self, flag_bytes: &[Byte]) -> Result<Vec<Byte>> {
        let mut buffer = Cursor::new(Vec::new());

        // Write TopHeader
        buffer.write_all(FILE_KEY)?;
        buffer.write_all(&K_CURR_VERSION.to_le_bytes())?;
        buffer.write_all(&0u32.to_le_bytes())?; // checksum

        // Write Header1
        let blob_size_pos = buffer.position() as usize;
        buffer.write_all(&0i64.to_le_bytes())?; // blob_size (placeholder)
        buffer.write_all(&(flag_bytes.len() as u32).to_le_bytes())?; // num_points
        buffer.write_all(&[self.compression_method as u8])?;
        buffer.write_all(&[self.min_value])?;
        buffer.write_all(&0u16.to_le_bytes())?; // reserved

        // Encode data
        if self.compression_method == CompressionMethod::BitStuff {
            let data_vec: Vec<u32> = flag_bytes.iter().map(|&b| b as u32 - self.min_value as u32).collect();

            let encoded = BitStuffer2::encode_simple(&data_vec)?;
            buffer.write_all(&encoded)?;
        } else if self.compression_method == CompressionMethod::HuffmanCodec {
            let mut huffman = Huffman::new();
            let encoded = huffman.encode(flag_bytes)?;
            buffer.write_all(&encoded)?;
        }

        // Update blob_size
        let mut result = buffer.into_inner();
        let blob_size = result.len() as i64;
        result[blob_size_pos..blob_size_pos + 8].copy_from_slice(&blob_size.to_le_bytes());

        // Compute and write checksum
        let checksum = compute_checksum_fletcher32(&result[16..blob_size as usize]);
        result[12..16].copy_from_slice(&checksum.to_le_bytes());

        Ok(result)
    }
}

pub struct FlagBytesDecoder;

impl FlagBytesDecoder {
    pub fn get_blob_size(data: &[Byte]) -> Result<u32> {
        if data.len() < 24 {
            return Err(LepccError::BufferTooSmall {
                needed: 24,
                provided: data.len(),
            });
        }

        if &data[0..10] != FILE_KEY {
            return Err(LepccError::NotLepcc("Invalid file key".to_string()));
        }

        let blob_size = i64::from_le_bytes([data[16], data[17], data[18], data[19], data[20], data[21], data[22], data[23]]);
        if blob_size < 0 || blob_size > u32::MAX as i64 {
            return Err(LepccError::Failed("Invalid blob size".to_string()));
        }

        Ok(blob_size as u32)
    }

    pub fn get_num_points(data: &[Byte]) -> Result<u32> {
        if data.len() < HEADER_SIZE {
            return Err(LepccError::BufferTooSmall {
                needed: HEADER_SIZE,
                provided: data.len(),
            });
        }

        let num_points = u32::from_le_bytes([data[16], data[17], data[18], data[19]]);
        Ok(num_points)
    }

    pub fn decode(data: &[Byte]) -> Result<Vec<Byte>> {
        let blob_size = Self::get_blob_size(data)? as usize;
        if data.len() < blob_size {
            return Err(LepccError::BufferTooSmall {
                needed: blob_size,
                provided: data.len(),
            });
        }

        // Verify checksum
        let checksum = compute_checksum_fletcher32(&data[16..blob_size]);
        let stored_checksum = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);
        if checksum != stored_checksum {
            return Err(LepccError::WrongChecksum {
                expected: stored_checksum,
                found: checksum,
            });
        }

        let num_elem = u32::from_le_bytes([data[16], data[17], data[18], data[19]]) as usize;
        let compression_method = CompressionMethod::try_from(data[22])?;

        let mut result = Vec::with_capacity(num_elem);
        let data_start = HEADER_SIZE;

        if compression_method == CompressionMethod::BitStuff {
            let min_value = data[23];
            let decoded = BitStuffer2::decode(&data[data_start..blob_size])?;

            for val in decoded.iter().take(num_elem) {
                result.push((*val as u8).wrapping_add(min_value));
            }
        } else if compression_method == CompressionMethod::HuffmanCodec {
            let huffman = Huffman::new();
            let decoded = huffman.decode(&data[data_start..blob_size], num_elem)?;
            result = decoded;
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_flag_bytes_encoding() {
        let flag_bytes = vec![1u8, 2, 3, 1, 2, 3];

        let mut encoder = FlagBytesEncoder::new();
        let _size = encoder.compute_num_bytes_needed(&flag_bytes).unwrap();
        let encoded = encoder.encode(&flag_bytes).unwrap();

        let num_points = FlagBytesDecoder::get_num_points(&encoded).unwrap();
        assert_eq!(num_points, flag_bytes.len() as u32);
    }

    #[test]
    fn test_flag_bytes_roundtrip() {
        let flag_bytes = vec![0u8, 1, 2, 3, 127, 255];

        let mut encoder = FlagBytesEncoder::new();
        let _size = encoder.compute_num_bytes_needed(&flag_bytes).unwrap();
        let encoded = encoder.encode(&flag_bytes).unwrap();

        let decoded = FlagBytesDecoder::decode(&encoded).unwrap();
        assert_eq!(decoded.len(), flag_bytes.len());
    }

    #[test]
    fn test_invalid_file_key() {
        let invalid_data = vec![0u8; 100];
        let result = FlagBytesDecoder::get_blob_size(&invalid_data);
        assert!(result.is_err());
    }
}
