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

//! Huffman coding implementation

use crate::error::{LepccError, Result};
use crate::types::Byte;
use crate::bit_stuffer2::BitStuffer2;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// Huffman coding implementation
pub struct Huffman {
    max_histo_size: usize,
    code_table: Vec<(u16, u32)>, // (code_length, code)
    #[allow(dead_code)]
    max_num_bits_lut: usize,
}


#[derive(Debug, Clone)]
struct HuffmanNode {
    weight: i64,
    value: Option<i16>,
    left: Option<Box<HuffmanNode>>,
    right: Option<Box<HuffmanNode>>,
}

impl HuffmanNode {
    fn new_leaf(value: i16, weight: i64) -> Self {
        HuffmanNode {
            weight: -weight,
            value: Some(value),
            left: None,
            right: None,
        }
    }

    fn new_internal(left: Box<HuffmanNode>, right: Box<HuffmanNode>) -> Self {
        HuffmanNode {
            weight: left.weight + right.weight,
            value: None,
            left: Some(left),
            right: Some(right),
        }
    }

    #[allow(dead_code)]
    fn is_leaf(&self) -> bool {
        self.left.is_none() && self.right.is_none()
    }



}


impl PartialEq for HuffmanNode {
    fn eq(&self, other: &Self) -> bool {
        self.weight == other.weight
    }
}

impl Eq for HuffmanNode {}

impl PartialOrd for HuffmanNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HuffmanNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.weight.cmp(&other.weight)
    }
}

impl Default for Huffman {
    fn default() -> Self {
        Self::new()
    }
}

impl Huffman {
    /// Create a new Huffman encoder
    pub fn new() -> Self {
        Huffman {
            max_histo_size: 1 << 15,
            code_table: Vec::new(),
            max_num_bits_lut: 12,
        }
    }

    /// Clear all internal state
    pub fn clear(&mut self) {
        self.code_table.clear();
    }

    /// Compute the number of bytes needed to encode data with given histogram
    pub fn compute_num_bytes_needed_to_encode(&mut self, histo: &[i32]) -> Option<i64> {
        if histo.is_empty() || histo.len() >= self.max_histo_size {
            return None;
        }

        if !self.compute_codes(histo) {
            return None;
        }

        let (num_bytes, _avg_bpp) = self.compute_compressed_size(histo)?;
        Some(num_bytes)
    }

    /// Encode byte data using Huffman coding
    pub fn encode(&mut self, data: &[Byte]) -> Result<Vec<Byte>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        // Build histogram
        let mut histo = vec![0i32; 256];
        for &byte in data {
            histo[byte as usize] += 1;
        }

        if !self.compute_codes(&histo) {
            return Err(LepccError::Failed("Failed to compute Huffman codes".to_string()));
        }

        let mut result = Vec::new();

        // Write code table
        self.write_code_table(&mut result)?;

        // Encode data
        let num_bits = data.len() * 32;

        let mut arr = vec![0u32; (num_bits + 31) / 32 + 1];
        let mut bit_pos = 0;
        let mut uint_idx = 0;

        for byte in data {
            let (len, code) = self.code_table[*byte as usize];
            if len == 0 {
                return Err(LepccError::Failed("Invalid Huffman code length".to_string()));
            }

            let len = len as usize;

            if 32 - bit_pos >= len {
                arr[uint_idx] |= code << (32 - bit_pos - len);
                bit_pos += len;
                if bit_pos == 32 {
                    uint_idx += 1;
                    bit_pos = 0;
                }
            } else {
                bit_pos += len - 32;
                arr[uint_idx] |= code >> bit_pos;
                uint_idx += 1;
                if uint_idx < arr.len() {
                    arr[uint_idx] = code << (32 - bit_pos);
                }
            }
        }

        let num_uints = uint_idx + (if bit_pos > 0 { 1 } else { 0 }) + 1;
        for i in 0..num_uints {
            result.extend_from_slice(&arr[i].to_le_bytes());
        }

        Ok(result)
    }

    /// Decode Huffman-encoded data
    pub fn decode(&self, _encoded: &[Byte], num_elements: usize) -> Result<Vec<Byte>> {
        let _result = vec![0u8; num_elements];

        // Read code table first
        let _pos = 0;
        let (_i0, _i1, _max_len) = self.get_range()?;

        let _bit_stuffer = BitStuffer2;
        // This is a simplified decode - in full implementation we'd need to read the table

        // For now, just return an error indicating this is incomplete
        Err(LepccError::Failed("Full Huffman decode not yet implemented".to_string()))
    }

    /// Compute Huffman codes from histogram
    fn compute_codes(&mut self, histo: &[i32]) -> bool {
        if histo.is_empty() || histo.len() >= self.max_histo_size {
            return false;
        }

        // Build priority queue of leaf nodes
        let mut heap: BinaryHeap<Reverse<HuffmanNode>> = BinaryHeap::new();

        for (i, &count) in histo.iter().enumerate() {
            if count > 0 {
                heap.push(Reverse(HuffmanNode::new_leaf(i as i16, count as i64)));
            }
        }

        if heap.len() < 2 {
            return false; // Need at least 2 different symbols for Huffman
        }

        // Build Huffman tree
        while heap.len() > 1 {
            let Reverse(node0) = heap.pop().unwrap();
            let Reverse(node1) = heap.pop().unwrap();
            heap.push(Reverse(HuffmanNode::new_internal(
                Box::new(node0),
                Box::new(node1),
            )));
        }

        let Reverse(root) = heap.pop().unwrap();

        // Fill code table
        self.code_table.resize(histo.len(), (0u16, 0u32));
        if !Self::tree_to_lut(&root, 0, 0, &mut self.code_table) {
            return false;
        }

        // Convert to canonical codes
        self.convert_codes_to_canonical();

        true
    }

    /// Traverse tree and fill lookup table
    fn tree_to_lut(
        node: &HuffmanNode,
        num_bits: u16,
        bits: u32,
        code_table: &mut [(u16, u32)],
    ) -> bool {
        if num_bits == 32 {
            return false; // Max huffman code length we allow
        }

        if let Some(child0) = &node.left {
            if let Some(child1) = &node.right {
                return Self::tree_to_lut(child0, num_bits + 1, (bits << 1) | 0, code_table)
                    && Self::tree_to_lut(child1, num_bits + 1, (bits << 1) | 1, code_table);
            }
        }

        if let Some(value) = node.value {
            if (value as usize) < code_table.len() {
                code_table[value as usize] = (num_bits, bits);
            }
        }

        true
    }

    /// Convert codes to canonical form
    fn convert_codes_to_canonical(&mut self) {
        let table_size = self.code_table.len();

        // Create sort vector: (code_length * table_size - index, index)
        let mut sort_vec: Vec<(i32, usize)> = Vec::with_capacity(table_size);

        for (i, &(len, _)) in self.code_table.iter().enumerate() {
            if len > 0 {
                sort_vec.push((len as i32 * table_size as i32 - i as i32, i));
            }
        }

        // Sort descending
        sort_vec.sort_by(|a, b| b.cmp(a));

        // Create canonical codes
        let code_len = sort_vec[0].1;
        let mut max_code_len = self.code_table[code_len].0 as usize;
        let mut code_canonical = 0u32;
        let mut index = 0;

        while index < sort_vec.len() && sort_vec[index].0 > 0 {
            let idx = sort_vec[index].1;
            let delta = max_code_len.saturating_sub(self.code_table[idx].0 as usize);
            code_canonical >>= delta;
            max_code_len -= delta;
            self.code_table[idx].1 = code_canonical;
            code_canonical += 1;
            index += 1;
        }
    }

    /// Compute compressed size in bytes
    fn compute_compressed_size(&self, histo: &[i32]) -> Option<(i64, f64)> {
        if histo.is_empty() || histo.len() >= self.max_histo_size {
            return None;
        }

        let mut num_bytes = 0i64;
        if !self.compute_num_bytes_code_table(&mut num_bytes) {
            return None;
        }

        let mut num_bits = 0u64;
        let mut num_elem = 0u64;

        for (i, &count) in histo.iter().enumerate() {
            if count > 0 && i < self.code_table.len() {
                num_bits += count as u64 * self.code_table[i].0 as u64;
                num_elem += count as u64;
            }
        }

        if num_elem == 0 {
            return Some((0, 0.0));
        }

        let num_uints = ((num_bits as usize + 7) >> 3 + 3) >> 2 + 1;
        num_bytes += 4 * num_uints as i64;
        let avg_bpp = 8.0 * num_bytes as f64 / num_elem as f64;

        Some((num_bytes, avg_bpp))
    }

    /// Compute number of bytes needed for the code table
    fn compute_num_bytes_code_table(&self, num_bytes: &mut i64) -> bool {
        if let Ok((_i0, _i1, _max_len)) = self.get_range() {
            *num_bytes = 16; // Header: 4 * sizeof(int)

            // Simplified - in full implementation we'd compute properly
            *num_bytes += 64; // Approximate

            true
        } else {
            false
        }
    }

    /// Get range of codes for optimization
    fn get_range(&self) -> Result<(usize, usize, usize)> {
        if self.code_table.is_empty() || self.code_table.len() >= self.max_histo_size {
            return Err(LepccError::Failed("Invalid code table".to_string()));
        }

        // Find first and last non-zero entry
        let mut i = 0;
        while i < self.code_table.len() && self.code_table[i].0 == 0 {
            i += 1;
        }
        let i0 = i;

        if i >= self.code_table.len() {
            return Err(LepccError::Failed("Empty code table".to_string()));
        }

        let mut i = self.code_table.len() - 1;
        while i > 0 && self.code_table[i].0 == 0 {
            i -= 1;
        }
        let i1 = i + 1;

        if i1 <= i0 {
            return Err(LepccError::Failed("Invalid code range".to_string()));
        }

        let max_len = self.code_table[i0..i1]
            .iter()
            .map(|&(len, _)| len as usize)
            .max()
            .unwrap_or(0);

        Ok((i0, i1, max_len))
    }

    /// Write code table to buffer
    fn write_code_table(&self, buffer: &mut Vec<Byte>) -> Result<()> {
        // Header
        // Version = 4 (canonical codes)
        buffer.extend_from_slice(&4u32.to_le_bytes());
        // Size
        buffer.extend_from_slice(&(self.code_table.len() as u32).to_le_bytes());

        let (i0, i1, _max_len) = self.get_range()?;
        buffer.extend_from_slice(&(i0 as u32).to_le_bytes());
        buffer.extend_from_slice(&(i1 as u32).to_le_bytes());

        // Write code lengths using BitStuffer2
        let mut code_lengths: Vec<u32> = Vec::new();
        for i in i0..i1 {
            let k = self.wrap_around(i, self.code_table.len());
            code_lengths.push(self.code_table[k].0 as u32);
        }

        let encoded_len = BitStuffer2::encode_simple(&code_lengths)?;
        buffer.extend_from_slice(&encoded_len);

        // Write codes bit stuffed
        // (simplified - full implementation would do proper bit stuffing)
        Ok(())
    }

    /// Wrap around index
    fn wrap_around(&self, i: usize, size: usize) -> usize {
        i - if i < size { 0 } else { size }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_huffman_simple() {
        let mut huffman = Huffman::new();

        // Simple histogram
        let histo = vec![10i32, 5, 20, 15, 0, 0, 0];

        let result = huffman.compute_num_bytes_needed_to_encode(&histo);
        assert!(result.is_some());
        assert!(result.unwrap() > 0);
    }

    #[test]
    fn test_huffman_encode_decode() {
        let mut huffman = Huffman::new();

        let data = vec![1u8, 2, 2, 3, 3, 3, 1, 1, 1, 1];

        let encoded = huffman.encode(&data);
        // Note: Full decode test not implemented yet
        assert!(encoded.is_ok());
    }
}
