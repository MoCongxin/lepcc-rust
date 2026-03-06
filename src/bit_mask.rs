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

//! BitMask - Convenient and fast access to binary mask bits

use crate::types::Byte;

/// BitMask for efficient bit-level storage and access
#[derive(Clone)]
pub struct BitMask {
    bits: Vec<Byte>,
    n_cols: usize,
    n_rows: usize,
}

impl BitMask {
    /// Create a new empty BitMask
    pub fn new() -> Self {
        BitMask {
            bits: Vec::new(),
            n_cols: 0,
            n_rows: 0,
        }
    }

    /// Create a new BitMask with the specified size
    pub fn with_size(n_cols: usize, n_rows: usize) -> Self {
        let num_bytes = (n_cols * n_rows + 7) >> 3;
        BitMask {
            bits: vec![0; num_bytes],
            n_cols,
            n_rows,
        }
    }

    /// Get the column count
    pub fn width(&self) -> usize {
        self.n_cols
    }

    /// Get the row count
    pub fn height(&self) -> usize {
        self.n_rows
    }

    /// Get the number of bytes used
    pub fn size(&self) -> usize {
        (self.n_cols * self.n_rows + 7) >> 3
    }

    /// Get the raw bits slice
    pub fn bits(&self) -> &[Byte] {
        &self.bits
    }

    /// Get mutable reference to bits
    pub fn bits_mut(&mut self) -> &mut [Byte] {
        &mut self.bits
    }

    /// Get the bit value for the given index (1: valid, 0: not valid)
    pub fn is_valid(&self, k: usize) -> bool {
        if k >= self.n_cols * self.n_rows {
            return false;
        }
        (self.bits[k >> 3] & Self::bit(k)) != 0
    }

    /// Get the bit value for the given row and col
    pub fn is_valid_rc(&self, row: usize, col: usize) -> bool {
        self.is_valid(row * self.n_cols + col)
    }

    /// Set the bit as valid (1) at the given index
    pub fn set_valid(&mut self, k: usize) {
        if k < self.n_cols * self.n_rows {
            self.bits[k >> 3] |= Self::bit(k);
        }
    }

    /// Set the bit as valid (1) at the given flattened 2D index (row * n_cols + col)
    pub fn set_valid_2d(&mut self, row: usize, col: usize) {
        self.set_valid(row * self.n_cols + col);
    }

    /// Set the bit as invalid (0) at the given index
    pub fn set_invalid(&mut self, k: usize) {
        if k < self.n_cols * self.n_rows {
            self.bits[k >> 3] &= !Self::bit(k);
        }
    }

    /// Set the bit as invalid (0) at the given row and col
    pub fn set_invalid_rc(&mut self, row: usize, col: usize) {
        self.set_invalid(row * self.n_cols + col);
    }

    /// Set all bits to valid (1)
    pub fn set_all_valid(&mut self) {
        self.bits.fill(0xFF);
    }

    /// Set all bits to invalid (0)
    pub fn set_all_invalid(&mut self) {
        self.bits.fill(0);
    }

    /// Resize the BitMask
    pub fn set_size(&mut self, n_cols: usize, n_rows: usize) {
        if n_cols != self.n_cols || n_rows != self.n_rows {
            let num_bytes = (n_cols * n_rows + 7) >> 3;
            self.bits = vec![0; num_bytes];
            self.n_cols = n_cols;
            self.n_rows = n_rows;
        }
    }

    /// Count the number of valid bits (1s)
    pub fn count_valid_bits(&self) -> usize {
        const NUM_BITS_HEX: [u8; 16] = [0, 1, 1, 2, 1, 2, 2, 3, 1, 2, 2, 3, 2, 3, 3, 4];
        let mut sum = 0;

        for &byte in self.bits.iter() {
            sum += (NUM_BITS_HEX[(byte & 0x0F) as usize] + NUM_BITS_HEX[(byte >> 4) as usize]) as usize;
        }

        // Subtract undefined bits in the last byte
        let total_bits = self.n_cols * self.n_rows;
        let byte_bits = self.bits.len() * 8;

        for k in total_bits..byte_bits {
            if self.is_valid(k) {
                sum -= 1;
            }
        }

        sum
    }

    /// Find the next valid bit starting from k (inclusive)
    /// Returns -1 if there is no valid bit after k
    pub fn next_valid_bit(&self, k: i64) -> i64 {
        let total = self.n_cols * self.n_rows;

        if k < 0 || k as usize >= total {
            return -1;
        }

        let k = k as usize;
        let mut byte = self.bits[k >> 3] & (0xFF >> (k & 7));

        if byte == 0 {
            // Move along the bytes until we hit something
            let mut i = (k >> 3) + 1;
            let num_bytes = self.bits.len();

            while i < num_bytes && self.bits[i] == 0 {
                i += 1;
            }

            if i >= num_bytes {
                return -1;
            }

            let _k_new = i << 3;
            byte = self.bits[i];
        }

        // Search this byte starting at k
        let mut k_curr = k;
        let k_end = std::cmp::min(k + 8, total);

        while k_curr < k_end && (byte & Self::bit(k_curr)) == 0 {
            k_curr += 1;
        }

        if k_curr < k_end {
            k_curr as i64
        } else {
            -1
        }
    }

    /// Get the bit mask for a given bit position
    #[inline]
    fn bit(k: usize) -> Byte {
        (1 << 7) >> (k & 7)
    }

    /// Clear the BitMask
    pub fn clear(&mut self) {
        self.bits.clear();
        self.n_cols = 0;
        self.n_rows = 0;
    }
}

impl Default for BitMask {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitmask_basic() {
        let mut bm = BitMask::with_size(10, 10);
        bm.set_valid(5);
        bm.set_valid(15);
        assert!(bm.is_valid(5));
        assert!(bm.is_valid(15));
        assert!(!bm.is_valid(0));
    }

    #[test]
    fn test_bitmask_set_all() {
        let mut bm = BitMask::with_size(10, 10);
        bm.set_all_valid();
        for i in 0..100 {
            assert!(bm.is_valid(i));
        }
    }

    #[test]
    fn test_bitmask_count_valid_bits() {
        let mut bm = BitMask::with_size(10, 10);
        assert_eq!(bm.count_valid_bits(), 0);

        bm.set_valid(0);
        bm.set_valid(5);
        bm.set_valid(10);
        assert_eq!(bm.count_valid_bits(), 3);
    }

    #[test]
    fn test_bitmask_next_valid_bit() {
        let mut bm = BitMask::with_size(100, 1);
        bm.set_valid(5);
        bm.set_valid(23);
        bm.set_valid(67);

        assert_eq!(bm.next_valid_bit(0), 5);
        assert_eq!(bm.next_valid_bit(5), 5);
        assert_eq!(bm.next_valid_bit(6), 23);
        assert_eq!(bm.next_valid_bit(24), 67);
        assert_eq!(bm.next_valid_bit(68), -1);
    }
}
