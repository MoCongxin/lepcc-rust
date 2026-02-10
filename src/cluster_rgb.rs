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

//! ClusterRGB - Color compression using quantization/clustering

use crate::bit_mask::BitMask;
use crate::common::compute_checksum_fletcher32;
use crate::error::{LepccError, Result};
use crate::types::{Byte, RGB};
use std::io::{Cursor, Write};

const FILE_KEY: &[u8; 10] = b"ClusterRGB";
const K_CURR_VERSION: u16 = 1;
const HEADER_SIZE: usize = 32; // TopHeader(16) + Header1(16)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColorLookupMethod {
    None = 0,
    Lossless = 1,
    Array3D = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColorIndexCompressionMethod {
    NoCompression = 0,
    AllConst = 1,
    HuffmanCodec = 2,
}

pub struct ClusterRgbEncoder {
    max_num_colors: i32,
    color_lookup_method: ColorLookupMethod,
    color_index_compression_method: ColorIndexCompressionMethod,
    num_points: usize,
    color_map: Vec<RgbColor>,
    rgb_vec: Vec<RGB>,
    color_index_vec: Vec<Byte>,
}

#[derive(Debug, Clone, Copy, Default)]
struct RgbColor {
    r: Byte,
    g: Byte,
    b: Byte,
    a: Byte,
}

impl Default for ClusterRgbEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl ClusterRgbEncoder {
    pub fn new() -> Self {
        ClusterRgbEncoder {
            max_num_colors: 256,
            color_lookup_method: ColorLookupMethod::None,
            color_index_compression_method: ColorIndexCompressionMethod::NoCompression,
            num_points: 0,
            color_map: Vec::new(),
            rgb_vec: Vec::new(),
            color_index_vec: Vec::new(),
        }
    }

    /// Compute the number of bytes needed to encode RGB colors
    pub fn compute_num_bytes_needed(&mut self, colors: &[RGB]) -> Result<i64> {
        if colors.is_empty() {
            return Err(LepccError::WrongParam("No colors provided".to_string()));
        }

        self.num_points = colors.len();

        // Count colors
        let mut true_color_mask = BitMask::with_size(256 * 256, 256);
        true_color_mask.set_all_invalid();

        let mut num_orig_colors = 0usize;
        let mut lossless_colors: Vec<i32> = Vec::new();

        // Build histogram
        let mut level_6_histo = vec![0i32; 64 * 64 * 64];

        let num_color_steps = 1 << 6;
        let color_shift_6 = 8 - 6;

        for &rgb in colors {
            // Count original colors
            let n = (rgb.r as i32) << 8 | rgb.g as i32;
            let k = n + (rgb.b as i32) * 256 * 256;
            if !true_color_mask.is_valid(k as usize) {
                true_color_mask.set_valid(k as usize);
                num_orig_colors += 1;
                if num_orig_colors <= self.max_num_colors as usize {
                    let index = self.compute_3d_array_index(rgb.r, rgb.g, rgb.b, 8);
                    lossless_colors.push(index);
                }
            }

            // Update level 6 histogram
            let m = self.compute_3d_array_index(rgb.r, rgb.g, rgb.b, 6);
            if (m as usize) < level_6_histo.len() {
                level_6_histo[m as usize] += 1;
            }
        }

        let header_size = HEADER_SIZE as i64;

        // Decide compression method
        if 2 * self.num_points <= 3 * num_orig_colors.min(self.max_num_colors as usize) {
            // Store colors raw, no colormap
            self.color_lookup_method = ColorLookupMethod::None;
            self.rgb_vec = colors.to_vec();
            return Ok(header_size + (self.num_points * 3) as i64);
        } else if num_orig_colors <= self.max_num_colors as usize {
            // Lossless colormap
            self.color_lookup_method = ColorLookupMethod::Lossless;
            self.generate_colormap_lossless(&lossless_colors)?;
            self.turn_colors_to_indexes(colors)?;

            let mut index_bytes = self.compute_num_bytes_needed_color_indexes();
            if index_bytes < 0 {
                // Store raw
                self.color_lookup_method = ColorLookupMethod::None;
                self.rgb_vec = colors.to_vec();
                return Ok(header_size + (self.num_points * 3) as i64);
            }

            return Ok(header_size + (num_orig_colors * 3) as i64 + index_bytes);
        } else {
            // Lossy colormap using median cut
            self.color_lookup_method = ColorLookupMethod::Array3D;

            // Simplified: use a subset of colors as palette
            let mut color_map: Vec<RgbColor> = Vec::new();
            color_map.push(RgbColor::default());

            let mut seen_colors: std::collections::HashSet<(u8, u8, u8)> = std::collections::HashSet::new();

            'outer: for &rgb in colors {
                if seen_colors.contains(&(rgb.r, rgb.g, rgb.b)) {
                    continue;
                }

                if color_map.len() >= self.max_num_colors as usize {
                    break 'outer;
                }

                color_map.push(RgbColor {
                    r: rgb.r,
                    g: rgb.g,
                    b: rgb.b,
                    a: 0,
                });
                seen_colors.insert((rgb.r, rgb.g, rgb.b));
            }

            self.color_map = color_map;
            self.turn_colors_to_indexes(colors)?;

            let mut index_bytes = self.compute_num_bytes_needed_color_indexes();
            if index_bytes < 0 {
                self.color_lookup_method = ColorLookupMethod::None;
                self.rgb_vec = colors.to_vec();
                return Ok(header_size + (self.num_points * 3) as i64);
            }

            return Ok(header_size + (self.color_map.len() * 3) as i64 + index_bytes);
        }
    }

    /// Encode RGB colors
    pub fn encode(&self) -> Result<Vec<Byte>> {
        let mut buffer = Cursor::new(Vec::new());

        // Write TopHeader
        buffer.write_all(FILE_KEY)?;
        buffer.write_all(&K_CURR_VERSION.to_le_bytes())?;
        buffer.write_all(&0u32.to_le_bytes())?; // checksum

        // Write Header1
        let blob_size_pos = buffer.position() as usize;
        buffer.write_all(&0i64.to_le_bytes())?; // blob_size (placeholder)

        let num_points = match self.color_lookup_method {
            ColorLookupMethod::None => self.rgb_vec.len(),
            _ => self.color_index_vec.len(),
        };

        let num_colors = match self.color_lookup_method {
            ColorLookupMethod::None => 0,
            _ => self.color_map.len(),
        };

        buffer.write_all(&(num_points as u32).to_le_bytes())?;
        buffer.write_all(&(num_colors as u16).to_le_bytes())?;
        buffer.write_all(&[self.color_lookup_method as u8])?;
        buffer.write_all(&[self.color_index_compression_method as u8])?;

        // Write colormap if needed
        if self.color_lookup_method != ColorLookupMethod::None {
            for color in &self.color_map {
                buffer.write_all(&[color.r, color.g, color.b])?;
            }

            // Write color indexes
            if self.color_index_compression_method == ColorIndexCompressionMethod::NoCompression {
                buffer.write_all(&self.color_index_vec)?;
            } else if self.color_index_compression_method == ColorIndexCompressionMethod::AllConst {
                // No data needed - all points have same color
            } else {
                return Err(LepccError::Failed(
                    "Huffman codec for RGB not implemented".to_string(),
                ));
            }
        } else {
            // Write colors raw per point
            for rgb in &self.rgb_vec {
                buffer.write_all(&[rgb.r, rgb.g, rgb.b])?;
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

    fn compute_3d_array_index(&self, r: Byte, g: Byte, b: Byte, level: i32) -> i32 {
        let shift = 8 - level;
        (((r >> shift) << (level * 2)) + ((g >> shift) << level) + (b >> shift)) as i32
    }

    fn generate_colormap_lossless(&mut self, lossless_colors: &[i32]) -> Result<()> {
        self.color_map = Vec::with_capacity(lossless_colors.len());

        for &n in lossless_colors {
            let r = ((n >> 16) & 255) as u8;
            let g = ((n >> 8) & 255) as u8;
            let b = (n & 255) as u8;

            self.color_map.push(RgbColor { r, g, b, a: 0 });
        }

        Ok(())
    }

    fn turn_colors_to_indexes(&mut self, colors: &[RGB]) -> Result<()> {
        if self.color_lookup_method == ColorLookupMethod::None {
            return Err(LepccError::WrongParam(
                "No colormap available".to_string(),
            ));
        }

        self.color_index_vec = Vec::with_capacity(colors.len());

        for rgb in colors {
            let index = self.find_color_index(rgb);
            if index >= 256 {
                // Store raw instead
                self.color_lookup_method = ColorLookupMethod::None;
                self.rgb_vec = colors.to_vec();
                return Ok(());
            }
            self.color_index_vec.push(index as u8);
        }

        Ok(())
    }

    fn find_color_index(&self, rgb: &RGB) -> i32 {
        if self.color_map.is_empty() {
            return 0;
        }

        // For lossless colormap, use direct lookup
        if self.color_lookup_method == ColorLookupMethod::Lossless {
            for (i, color) in self.color_map.iter().enumerate() {
                if color.r == rgb.r && color.g == rgb.g && color.b == rgb.b {
                    return i as i32;
                }
            }
        } else {
            // Find closest color
            let mut best_index = 0;
            let mut best_dist = i32::MAX;

            for (i, color) in self.color_map.iter().enumerate() {
                let dr = (color.r as i32 - rgb.r as i32).abs();
                let dg = (color.g as i32 - rgb.g as i32).abs();
                let db = (color.b as i32 - rgb.b as i32).abs();
                let dist = dr + dg + db;

                if dist < best_dist {
                    best_dist = dist;
                    best_index = i;
                }
            }

            return best_index as i32;
        }

        0
    }

    fn compute_num_bytes_needed_color_indexes(&mut self) -> i64 {
        if self.color_index_vec.is_empty() {
            return -1;
        }

        // Build histogram
        let mut histo = vec![0i32; 256];
        let mut num_non_zero_bins = 0i32;

        for &index in &self.color_index_vec {
            if histo[index as usize] == 0 {
                num_non_zero_bins += 1;
            }
            histo[index as usize] += 1;
        }

        // Decide compression method
        if num_non_zero_bins <= 1 {
            self.color_index_compression_method = ColorIndexCompressionMethod::AllConst;
            0
        } else {
            self.color_index_compression_method = ColorIndexCompressionMethod::NoCompression;
            self.color_index_vec.len() as i64
        }
    }
}

pub struct ClusterRgbDecoder;

impl ClusterRgbDecoder {
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

    pub fn decode(data: &[Byte]) -> Result<Vec<RGB>> {
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

        let num_points = u32::from_le_bytes([data[16], data[17], data[18], data[19]]) as usize;
        let num_colors = u16::from_le_bytes([data[20], data[21]]) as usize;
        let color_lookup_method = ColorLookupMethod::try_from(data[22])?;

        let mut result = Vec::with_capacity(num_points);

        if num_colors == 0 {
            // Colors stored raw per point
            let color_data_start = HEADER_SIZE;
            for i in 0..num_points {
                let idx = color_data_start + i * 3;
                if idx + 2 >= data.len() {
                    break;
                }
                result.push(RGB {
                    r: data[idx],
                    g: data[idx + 1],
                    b: data[idx + 2],
                });
            }
        } else {
            // Read colormap
            let mut color_map: Vec<RgbColor> = Vec::with_capacity(num_colors);
            let color_map_start = HEADER_SIZE;
            for i in 0..num_colors {
                let idx = color_map_start + i * 3;
                if idx + 2 >= data.len() {
                    break;
                }
                color_map.push(RgbColor {
                    r: data[idx],
                    g: data[idx + 1],
                    b: data[idx + 2],
                    a: 0,
                });
            }

            let compression_method = ColorIndexCompressionMethod::try_from(data[23])?;
            let indexes_start = HEADER_SIZE + num_colors * 3;

            if compression_method == ColorIndexCompressionMethod::NoCompression {
                for i in 0..num_points {
                    let idx = indexes_start + i;
                    if idx >= data.len() {
                        break;
                    }
                    let color_idx = data[idx] as usize;
                    if color_idx < color_map.len() {
                        let color = color_map[color_idx];
                        result.push(RGB {
                            r: color.r,
                            g: color.g,
                            b: color.b,
                        });
                    }
                }
            } else if compression_method == ColorIndexCompressionMethod::AllConst {
                if !color_map.is_empty() {
                    let color = color_map[0];
                    for _ in 0..num_points {
                        result.push(RGB {
                            r: color.r,
                            g: color.g,
                            b: color.b,
                        });
                    }
                }
            }
        }

        Ok(result)
    }
}

impl TryFrom<u8> for ColorLookupMethod {
    type Error = LepccError;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(ColorLookupMethod::None),
            1 => Ok(ColorLookupMethod::Lossless),
            2 => Ok(ColorLookupMethod::Array3D),
            _ => Err(LepccError::WrongParam(format!("Invalid color lookup method: {}", value))),
        }
    }
}

impl TryFrom<u8> for ColorIndexCompressionMethod {
    type Error = LepccError;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(ColorIndexCompressionMethod::NoCompression),
            1 => Ok(ColorIndexCompressionMethod::AllConst),
            2 => Ok(ColorIndexCompressionMethod::HuffmanCodec),
            _ => Err(LepccError::WrongParam(format!(
                "Invalid compression method: {}",
                value
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_rgb_encoding() {
        let colors = vec![
            RGB { r: 255, g: 0, b: 0 },
            RGB { r: 0, g: 255, b: 0 },
            RGB { r: 0, g: 0, b: 255 },
        ];

        let mut encoder = ClusterRgbEncoder::new();
        let _size = encoder.compute_num_bytes_needed(&colors).unwrap();
        let encoded = encoder.encode().unwrap();

        let num_points = ClusterRgbDecoder::get_num_points(&encoded).unwrap();
        assert_eq!(num_points, colors.len() as u32);
    }

    #[test]
    fn test_rgb_roundtrip() {
        let colors = vec![
            RGB { r: 255, g: 0, b: 0 },
            RGB { r: 0, g: 255, b: 0 },
            RGB { r: 0, g: 0, b: 255 },
        ];

        let mut encoder = ClusterRgbEncoder::new();
        let _size = encoder.compute_num_bytes_needed(&colors).unwrap();
        let encoded = encoder.encode().unwrap();

        let decoded = ClusterRgbDecoder::decode(&encoded).unwrap();
        assert_eq!(decoded.len(), colors.len());
    }

    #[test]
    fn test_invalid_file_key() {
        let invalid_data = vec![0u8; 100];
        let result = ClusterRgbDecoder::get_blob_size(&invalid_data);
        assert!(result.is_err());
    }
}
