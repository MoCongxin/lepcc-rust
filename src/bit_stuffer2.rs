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

//! BitStuffer2 - Bit level compression for unsigned integer arrays

use crate::error::Result;
use crate::types::Byte;

/// BitStuffer2 for lossless compression of unsigned integer arrays
pub struct BitStuffer2;

impl BitStuffer2 {
    /// Encode unsigned integers using simple bit-stuffing
    ///
    /// # Arguments
    ///
    /// * `data` - Slice of unsigned integers to encode
    ///
    /// # Returns
    ///
    /// Encoded byte vector
    pub fn encode_simple(data: &[u32]) -> Result<Vec<Byte>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        let max_elem = *data.iter().max().unwrap_or(&0);
        let num_bits = Self::bits_needed_32(max_elem);

        if num_bits >= 32 {
            return Err(crate::error::LepccError::Failed(
                "Too many bits needed for bit stuffing".to_string(),
            ));
        }

        let num_elements = data.len() as u32;
        let num_uints = (data.len() * num_bits as usize + 31) / 32;

        // Determine header byte
        let mut num_bits_byte = num_bits as u8;

        // Use upper 2 bits to encode the type used for num_elements: Byte, ushort, or uint
        let (nb, bits67) = Self::num_bytes_uint(num_elements);
        num_bits_byte |= bits67 << 6;

        // bit5 = 0 means simple mode

        let mut result = Vec::new();
        result.push(num_bits_byte);

        Self::encode_uint(&mut result, num_elements, nb);

        if num_uints > 0 {
            Self::bit_stuff(&mut result, data, num_bits);
        }

        Ok(result)
    }

    /// Decode unsigned integers from bit-stuffed data
    ///
    /// # Arguments
    ///
    /// * `data` - Encoded byte slice
    ///
    /// # Returns
    ///
    /// Decoded unsigned integer vector
    pub fn decode(data: &[Byte]) -> Result<Vec<u32>> {
        let mut pos = 0;

        if pos >= data.len() {
            return Err(crate::error::LepccError::WrongParam(
                "Empty input data".to_string(),
            ));
        }

        let num_bits_byte = data[pos];
        pos += 1;

        let bits67 = (num_bits_byte >> 6) & 0x3;
        let nb = match bits67 {
            0 => 4,
            1 => 2,
            2 => 1,
            3 => 4, // This shouldn't happen but C++ code handles it
            _ => unreachable!(),
        };

        let do_lut = (num_bits_byte & (1 << 5)) != 0;
        let num_bits = (num_bits_byte & 0x1F) as usize;

        // Decode number of elements
        let (num_elements, bytes_read) = Self::decode_uint(&data[pos..], nb)?;
        pos += bytes_read;

        let mut result_vec = Vec::new();

        if !do_lut {
            if num_bits > 0 {
                result_vec = Self::bit_unstuff(&data[pos..], num_elements as usize, num_bits)?;
            } else {
                // numBits = 0, all elements = 0
                result_vec.resize(num_elements as usize, 0);
            }
        } else {
            // LUT mode (not commonly used in simple case)
            return Err(crate::error::LepccError::Failed(
                "LUT mode not implemented in simple decode".to_string(),
            ));
        }

        Ok(result_vec)
    }

    /// Compute the number of bytes needed to encode data with simple bit-stuffing
    ///
    /// # Arguments
    ///
    /// * `num_elem` - Number of elements
    /// * `max_elem` - Maximum element value
    ///
    /// # Returns
    ///
    /// Number of bytes needed
    pub fn compute_num_bytes_needed_simple(num_elem: u32, max_elem: u32) -> usize {
        let num_bits = Self::bits_needed_32(max_elem);
        let nb = Self::num_bytes_uint(num_elem).0;
        1 + nb as usize + ((num_elem as usize * num_bits + 7) >> 3)
    }

    /// Compute number of bits needed to represent the value (0-31 for 32-bit)
    fn bits_needed_32(mut value: u32) -> usize {
        let mut num_bits = 0;
        while num_bits < 32 && value > 0 {
            value >>= 1;
            num_bits += 1;
        }
        num_bits
    }

    /// Get the number of bytes needed and the encoding for a u32 value
    fn num_bytes_uint(k: u32) -> (u8, u8) {
        let nb = if k < 256 {
            1
        } else if k < (1 << 16) {
            2
        } else {
            4
        };
        let bits67 = if nb == 4 { 0 } else { 3 - nb };
        (nb, bits67)
    }

    /// Encode a u32 into the buffer
    fn encode_uint(buffer: &mut Vec<Byte>, k: u32, nb: u8) {
        match nb {
            1 => buffer.push(k as u8),
            2 => {
                let k_short = k as u16;
                buffer.extend_from_slice(&k_short.to_le_bytes());
            }
            4 => buffer.extend_from_slice(&k.to_le_bytes()),
            _ => panic!("Invalid nb value: {}", nb),
        }
    }

    /// Decode a u32 from the buffer
    fn decode_uint(data: &[Byte], nb: u8) -> Result<(u32, usize)> {
        if data.len() < nb as usize {
            return Err(crate::error::LepccError::BufferTooSmall {
                needed: nb as usize,
                provided: data.len(),
            });
        }

        let result = match nb {
            1 => data[0] as u32,
            2 => u16::from_le_bytes([data[0], data[1]]) as u32,
            4 => u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
            _ => {
                return Err(crate::error::LepccError::WrongParam(
                    "Invalid nb value".to_string(),
                ))
            }
        };

        Ok((result, nb as usize))
    }

    /// Number of tail bytes not needed (optimization)
    fn num_tail_bytes_not_needed(num_elem: usize, num_bits: usize) -> usize {
        let num_bits_tail = (num_elem * num_bits) & 31;
        let num_bytes_tail = (num_bits_tail + 7) >> 3;
        if num_bytes_tail > 0 {
            4 - num_bytes_tail
        } else {
            0
        }
    }

    /// Bit-stuff the data into the output buffer
    fn bit_stuff(output: &mut Vec<Byte>, data: &[u32], num_bits: usize) {
        let num_elements = data.len();
        let num_uints = (num_elements * num_bits + 31) / 32;
        let num_bytes = num_uints * 4;

        // Allocate temporary buffer for uints
        let mut tmp_buffer = vec![0u32; num_uints];

        // Do the stuffing - use i32 for bit_pos to match C++ (can be negative)
        let mut dst_ptr = 0usize;
        let mut bit_pos: i32 = 0;
        let num_bits_i32 = num_bits as i32;

        for &val in data.iter() {
            let val = val as u32;

            if 32 - bit_pos >= num_bits_i32 {
                tmp_buffer[dst_ptr] |= val << (bit_pos as u32);
                bit_pos += num_bits_i32;
                if bit_pos == 32 {
                    dst_ptr += 1;
                    bit_pos = 0;
                }
            } else {
                // Match C++ logic: first OR, then increment dstPtr
                tmp_buffer[dst_ptr] |= val << (bit_pos as u32);
                dst_ptr += 1;
                tmp_buffer[dst_ptr] |= val >> ((32 - bit_pos) as u32);
                bit_pos += num_bits_i32 - 32;
            }
        }

        // Copy the bytes to the output
        let num_bytes_used = num_bytes - Self::num_tail_bytes_not_needed(num_elements, num_bits);

        // Convert u32 buffer to bytes
        for &uint_val in &tmp_buffer {
            output.extend_from_slice(&uint_val.to_le_bytes());
        }

        // Truncate to actual used bytes
        output.truncate(output.len() - (num_bytes - num_bytes_used));
    }

    /// Bit-unstuff the data from the input buffer
    fn bit_unstuff(input: &[Byte], num_elements: usize, num_bits: usize) -> Result<Vec<u32>> {
        if std::env::var("LEPCC_DEBUG").is_ok() {
            eprintln!("=== bit_unstuff called ===");
            eprintln!("  num_elements: {}", num_elements);
            eprintln!("  num_bits: {}", num_bits);
            eprintln!("  input bytes: {:02X?}", input);
        }

        if num_bits == 0 {
            // All elements are 0
            return Ok(vec![0u32; num_elements]);
        }

        let num_uints = (num_elements * num_bits + 31) / 32;
        let num_bytes_full = num_uints * 4;
        let num_bytes_used =
            num_bytes_full - Self::num_tail_bytes_not_needed(num_elements, num_bits);

        if std::env::var("LEPCC_DEBUG").is_ok() {
            eprintln!("  num_uints: {}", num_uints);
            eprintln!("  num_bytes_full: {}", num_bytes_full);
            eprintln!("  num_bytes_used: {}", num_bytes_used);
            eprintln!(
                "  tail_bytes_not_needed: {}",
                Self::num_tail_bytes_not_needed(num_elements, num_bits)
            );
        }

        if input.len() < num_bytes_used {
            return Err(crate::error::LepccError::BufferTooSmall {
                needed: num_bytes_used,
                provided: input.len(),
            });
        }

        // Convert bytes to u32 buffer
        let mut tmp_buffer = vec![0u32; num_uints];
        for (i, uint_val) in tmp_buffer.iter_mut().enumerate() {
            let byte_offset = i * 4;
            if byte_offset + 3 < input.len() {
                *uint_val = u32::from_le_bytes([
                    input[byte_offset],
                    input[byte_offset + 1],
                    input[byte_offset + 2],
                    input[byte_offset + 3],
                ]);
            } else if byte_offset < input.len() {
                // Partial read for the last u32
                let bytes_to_copy = input.len() - byte_offset;
                let mut bytes = [0u8; 4];
                bytes[0..bytes_to_copy].copy_from_slice(&input[byte_offset..]);
                *uint_val = u32::from_le_bytes(bytes);
            }
        }

        if std::env::var("LEPCC_DEBUG").is_ok() {
            eprintln!("  tmp_buffer after conversion: {:02X?}", tmp_buffer);
        }

        // Note: In C++, m_tmpBitStuffVec is a member variable that may contain
        // old data, so it sets the last uint to 0 BEFORE memcpy. In Rust,
        // tmp_buffer is already zero-initialized, so we don't need to do this.

        // Do the unstuffing (matching C++ implementation)
        let mut result = vec![0u32; num_elements];
        let mut src_ptr = 0usize;
        let mut bit_pos = 0usize;
        let nb = 32 - num_bits;

        if std::env::var("LEPCC_DEBUG").is_ok() {
            eprintln!("  nb: {}", nb);
            eprintln!("  Starting unstuffing loop...");
        }

        for i in 0..num_elements {
            // C++: if (nb - bitPos >= 0) - includes both nb > bitPos and nb == bitPos
            if nb >= bit_pos {
                result[i] = ((tmp_buffer[src_ptr] << (nb - bit_pos)) >> nb) as u32;
                bit_pos += num_bits;
                if bit_pos >= 32 {
                    src_ptr += 1;
                    bit_pos -= 32;
                }
            } else {
                // nb < bit_pos, need to read from next u32
                result[i] = (tmp_buffer[src_ptr] >> bit_pos) as u32;
                src_ptr += 1;
                result[i] |= ((tmp_buffer[src_ptr] << (64 - num_bits - bit_pos)) >> nb) as u32;
                bit_pos -= nb;
            }

            if std::env::var("LEPCC_DEBUG").is_ok() {
                eprintln!(
                    "  [{}] src_ptr={}, bit_pos={}, result={}",
                    i, src_ptr, bit_pos, result[i]
                );
            }
        }

        if std::env::var("LEPCC_DEBUG").is_ok() {
            eprintln!("  Final result: {:?}", result);
            eprintln!("=== bit_unstuff done ===");
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_stuffer_empty() {
        let data: Vec<u32> = vec![];
        let encoded = BitStuffer2::encode_simple(&data).unwrap();
        assert_eq!(encoded.len(), 0);
    }

    #[test]
    fn test_bit_stuffer_single_value() {
        let data = vec![123u32];
        let encoded = BitStuffer2::encode_simple(&data).unwrap();
        let decoded = BitStuffer2::decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_bit_stuffer_multiple_values() {
        let data = vec![0, 1, 2, 3, 4, 5, 6, 7];
        let encoded = BitStuffer2::encode_simple(&data).unwrap();
        let decoded = BitStuffer2::decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_bit_stuffer_large_values() {
        let data = vec![0, 100, 200, 300, 400, 500];
        let encoded = BitStuffer2::encode_simple(&data).unwrap();
        let decoded = BitStuffer2::decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_bit_stuffer_compression() {
        let data = vec![0u32; 1000];
        let encoded = BitStuffer2::encode_simple(&data).unwrap();
        // All zeros should compress well
        assert!(encoded.len() < data.len() * std::mem::size_of::<u32>());
    }

    #[test]
    fn test_bits_needed_32() {
        assert_eq!(BitStuffer2::bits_needed_32(0), 0);
        assert_eq!(BitStuffer2::bits_needed_32(1), 1);
        assert_eq!(BitStuffer2::bits_needed_32(2), 2);
        assert_eq!(BitStuffer2::bits_needed_32(3), 2);
        assert_eq!(BitStuffer2::bits_needed_32(4), 3);
        assert_eq!(BitStuffer2::bits_needed_32(7), 3);
        assert_eq!(BitStuffer2::bits_needed_32(8), 4);
        assert_eq!(BitStuffer2::bits_needed_32(255), 8);
        assert_eq!(BitStuffer2::bits_needed_32(256), 9);
    }

    #[test]
    fn test_compute_num_bytes_needed() {
        let num_elem = 100;
        let max_elem = 255;
        let bytes_needed = BitStuffer2::compute_num_bytes_needed_simple(num_elem, max_elem);
        assert!(bytes_needed > 0);
        assert!(bytes_needed < num_elem as usize * 4); // Should be compressed
    }
}
