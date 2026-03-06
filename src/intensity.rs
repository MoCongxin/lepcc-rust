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

//! Intensity compression implementation

use crate::bit_mask::BitMask;
use crate::bit_stuffer2::BitStuffer2;
use crate::common::compute_checksum_fletcher32;
use crate::error::{LepccError, Result};
use crate::types::Byte;
use std::io::{Cursor, Write};

const FILE_KEY: &[u8; 10] = b"Intensity ";
const K_CURR_VERSION: u16 = 1;
const HEADER_SIZE: usize = 32; // TopHeader(16) + Header1(16)

pub struct IntensityEncoder {
    upscale_factor: i32,
    num_bytes_needed: i64,
    bpp: i32,
    #[allow(dead_code)]
    data_vec: Vec<u32>,
}


impl Default for IntensityEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl IntensityEncoder {
    pub fn new() -> Self {
        IntensityEncoder {
            upscale_factor: 0,
            num_bytes_needed: 0,
            bpp: 0,
            data_vec: Vec::new(),
        }
    }

    /// Compute the number of bytes needed to encode intensity values
    pub fn compute_num_bytes_needed(&mut self, intensities: &[u16]) -> Result<i64> {
        if intensities.is_empty() {
            return Err(LepccError::WrongParam("No intensities provided".to_string()));
        }

        let max_elem = *intensities.iter().max().unwrap_or(&0);

        // Determine upscale factor if any
        self.upscale_factor = self.find_upscale_factor(intensities, max_elem);
        let max_elem = max_elem / self.upscale_factor as u16;

        // Determine bpp
        self.bpp = 0;
        while self.bpp < 16 && (max_elem >> self.bpp) > 0 {
            self.bpp += 1;
        }

        let header_size = HEADER_SIZE as i64;

        if self.bpp == 8 || self.bpp == 16 {
            self.num_bytes_needed = header_size + (intensities.len() * (self.bpp as usize / 8)) as i64;
        } else {
            let _bit_stuffer = BitStuffer2;
            self.num_bytes_needed = header_size
                + BitStuffer2::compute_num_bytes_needed_simple(intensities.len() as u32, max_elem as u32) as i64;
        }

        Ok(self.num_bytes_needed)
    }

    /// Encode intensity values
    pub fn encode(&self, intensities: &[u16]) -> Result<Vec<Byte>> {
        let mut buffer = Cursor::new(Vec::new());

        // Write TopHeader
        buffer.write_all(FILE_KEY)?;
        buffer.write_all(&K_CURR_VERSION.to_le_bytes())?;
        buffer.write_all(&0u32.to_le_bytes())?; // checksum

        // Write Header1
        let blob_size_pos = buffer.position() as usize;
        buffer.write_all(&0i64.to_le_bytes())?; // blob_size (placeholder)
        buffer.write_all(&(intensities.len() as u32).to_le_bytes())?; // num_points
        buffer.write_all(&(self.upscale_factor as u16).to_le_bytes())?;
        buffer.write_all(&[self.bpp as u8])?;
        buffer.write_all(&[0u8])?; // reserved

        // Encode intensity data
        if self.bpp == 16 {
            for &intensity in intensities {
                buffer.write_all(&intensity.to_le_bytes())?;
            }
        } else if self.bpp == 8 && self.upscale_factor == 1 {
            // Common case: 8-bit, no upscaling
            for &intensity in intensities {
                buffer.write_all(&[intensity as u8])?;
            }
        } else {
            let mut data_vec = vec![0u32; intensities.len()];

            if self.upscale_factor == 1 {
                for (i, &intensity) in intensities.iter().enumerate() {
                    data_vec[i] = intensity as u32;
                }
            } else {
                for (i, &intensity) in intensities.iter().enumerate() {
                    data_vec[i] = (intensity / self.upscale_factor as u16) as u32;
                }
            }

            if self.bpp == 8 {
                for val in &data_vec {
                    buffer.write_all(&[*val as u8])?;
                }
            } else {
                let encoded = BitStuffer2::encode_simple(&data_vec)?;
                buffer.write_all(&encoded)?;
            }
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

    fn find_upscale_factor(&self, intensities: &[u16], max_elem: u16) -> i32 {
        if max_elem == 0 {
            return 1;
        }

        let mut histo_mask = BitMask::with_size(1 + max_elem as usize, 1);
        histo_mask.set_all_invalid();

        for &intensity in intensities {
            histo_mask.set_valid(intensity as usize);
        }

        let mut k0 = histo_mask.next_valid_bit(0);
        if k0 < 0 {
            return 1;
        }

        #[allow(unused_assignments)]
        let mut k1 = k0;
        let mut min_delta = k0;

        // First pass: find min delta
        loop {
            let next = histo_mask.next_valid_bit(k0 + 1);
            if next < 0 {
                break;
            }
            k1 = next;
            min_delta = min_delta.min(k1 - k0);
            k0 = k1;
            if min_delta <= 1 {
                return 1;
            }
        }

        // Second pass: check all entries are multiples of min delta
        k0 = -1;
        loop {
            let next = histo_mask.next_valid_bit(k0 + 1);
            if next < 0 {
                break;
            }
            k1 = next;
            k0 = k1;
            if (k1 % min_delta) != 0 {
                return 1;
            }
        }

        min_delta as i32
    }
}

pub struct IntensityDecoder;

impl IntensityDecoder {
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

    pub fn decode(data: &[Byte]) -> Result<Vec<u16>> {
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
        let scale = u16::from_le_bytes([data[20], data[21]]);
        let bpp = data[22];

        if scale < 1 || bpp > 16 {
            return Err(LepccError::Failed("Invalid header values".to_string()));
        }

        let mut result = Vec::with_capacity(num_elem);
        let data_start = HEADER_SIZE;

        if bpp == 16 {
            for i in 0..num_elem {
                let idx = data_start + i * 2;
                if idx + 1 >= data.len() {
                    break;
                }
                result.push(u16::from_le_bytes([data[idx], data[idx + 1]]));
            }
        } else if bpp == 8 && scale == 1 {
            for i in 0..num_elem {
                let idx = data_start + i;
                if idx >= data.len() {
                    break;
                }
                result.push(data[idx] as u16);
            }
        } else {
            let data_slice = &data[data_start..blob_size];
            let decoded_u32 = BitStuffer2::decode(data_slice)?;

            for val in decoded_u32.iter().take(num_elem) {
                result.push((*val as u16) * scale);
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_intensity_encoding() {
        let intensities = vec![100u16, 200, 300, 150, 50];

        let mut encoder = IntensityEncoder::new();
        let _size = encoder.compute_num_bytes_needed(&intensities).unwrap();
        let encoded = encoder.encode(&intensities).unwrap();

        let num_points = IntensityDecoder::get_num_points(&encoded).unwrap();
        assert_eq!(num_points, intensities.len() as u32);
    }

    #[test]
    fn test_intensity_roundtrip() {
        let intensities = vec![100u16, 200, 300, 150, 50];

        let mut encoder = IntensityEncoder::new();
        let _size = encoder.compute_num_bytes_needed(&intensities).unwrap();
        let encoded = encoder.encode(&intensities).unwrap();

        let decoded = IntensityDecoder::decode(&encoded).unwrap();
        assert_eq!(decoded.len(), intensities.len());
    }

    #[test]
    fn test_invalid_file_key() {
        let invalid_data = vec![0u8; 100];
        let result = IntensityDecoder::get_blob_size(&invalid_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_upscale_factor() {
        let encoder = IntensityEncoder::new();

        // Intensities that are multiples of 10
        let intensities = vec![0u16, 10, 20, 30, 40];
        let factor = encoder.find_upscale_factor(&intensities, 40);
        assert_eq!(factor, 10);

        // Intensities with min delta of 1
        let intensities = vec![0u16, 1, 2, 3, 5, 10];
        let factor = encoder.find_upscale_factor(&intensities, 10);
        assert_eq!(factor, 1);
    }
}
