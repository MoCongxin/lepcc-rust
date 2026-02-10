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

//! Common utilities - Fletcher's checksum

use crate::types::Byte;

/// Compute Fletcher's checksum (32-bit version)
///
/// From https://en.wikipedia.org/wiki/Fletcher's_checksum
/// Modified from ushorts to bytes (by Lucian Plesea)
///
/// # Arguments
///
/// * `data` - Byte slice to compute checksum for
///
/// # Returns
///
/// 32-bit Fletcher checksum
pub fn compute_checksum_fletcher32(data: &[Byte]) -> u32 {
    let mut sum1: u32 = 0xFFFF;
    let mut sum2: u32 = 0xFFFF;

    let mut words = data.len() / 2;

    while words > 0 {
        let tlen = if words >= 359 { 359 } else { words as u32 };
        words -= tlen as usize;

        let mut len = tlen;
        let mut pos = 0;
        while len > 0 {
            sum1 = sum1.wrapping_add((data[pos] as u32) << 8);
            sum2 = sum2.wrapping_add(sum1);
            pos += 1;
            sum1 = sum1.wrapping_add(data[pos] as u32);
            sum2 = sum2.wrapping_add(sum1);
            pos += 1;
            len -= 1;
        }

        sum1 = (sum1 & 0xFFFF).wrapping_add(sum1 >> 16);
        sum2 = (sum2 & 0xFFFF).wrapping_add(sum2 >> 16);
    }

    // Add the straggler byte if it exists
    if data.len() & 1 != 0 {
        let last = data[data.len() - 1];
        sum1 = sum1.wrapping_add((last as u32) << 8);
        sum2 = sum2.wrapping_add(sum1);
    }

    // Second reduction step to reduce sums to 16 bits
    sum1 = (sum1 & 0xFFFF).wrapping_add(sum1 >> 16);
    sum2 = (sum2 & 0xFFFF).wrapping_add(sum2 >> 16);

    sum2 << 16 | sum1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fletcher32_empty() {
        let data: Vec<Byte> = Vec::new();
        let checksum = compute_checksum_fletcher32(&data);
        // Empty data should produce a known checksum
        assert_eq!(checksum, 0xFFFF0000);
    }

    #[test]
    fn test_fletcher32_simple() {
        let data: Vec<Byte> = vec![0xAA, 0xBB, 0xCC, 0xDD];
        let checksum = compute_checksum_fletcher32(&data);
        assert_ne!(checksum, 0);
    }

    #[test]
    fn test_fletcher32_consistency() {
        let data: Vec<Byte> = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05];
        let checksum1 = compute_checksum_fletcher32(&data);
        let checksum2 = compute_checksum_fletcher32(&data);
        assert_eq!(checksum1, checksum2);
    }

    #[test]
    fn test_fletcher32_odd_length() {
        let data: Vec<Byte> = vec![0xAA, 0xBB, 0xCC];
        let checksum = compute_checksum_fletcher32(&data);
        assert_ne!(checksum, 0);
    }
}
