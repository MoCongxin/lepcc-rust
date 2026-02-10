// Copyright 2016 Esri
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! LEPCC XYZ compression implementation - Simplified version for testing

use crate::bit_stuffer2::BitStuffer2;
use crate::common::compute_checksum_fletcher32;
use crate::error::{LepccError, Result};
use crate::types::{Byte, Extent3D, Point3D};
use std::io::{Cursor, Write};

const FILE_KEY: &[u8; 10] = b"LEPCC     ";
const K_CURR_VERSION: u16 = 1;
const HEADER_SIZE: usize = 104; // TopHeader(16) + Header1(88)

// Cell3D structure for quantized points
#[derive(Debug, Clone)]
struct Cell3D {
    x: i32,
    y: i32,
    z: i32,
    orig_pt_index: usize, // Original index before sorting
    xy_cell_index: i64,   // Computed index for sorting
}

impl Cell3D {
    fn new(x: i32, y: i32, z: i32, orig_pt_index: usize, nx: i32) -> Self {
        let xy_cell_index = (y as i64) * (nx as i64) + (x as i64);
        Cell3D {
            x,
            y,
            z,
            orig_pt_index,
            xy_cell_index,
        }
    }
}

// Comparison function for sorting
fn cell3d_compare(a: &Cell3D, b: &Cell3D) -> std::cmp::Ordering {
    a.xy_cell_index.cmp(&b.xy_cell_index)
}

pub struct LepccEncoder {
    section_size: i32,
    num_bytes_needed: i64,
    extent_3d: Extent3D,
    max_error: Point3D,

    // Internal state
    cell_3d_vec: Vec<Cell3D>,
    y_delta_vec: Vec<u32>,
    num_points_per_row_vec: Vec<u32>,
    x_delta_vec: Vec<u32>,
    z_vec: Vec<u32>,
}

impl Default for LepccEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl LepccEncoder {
    pub fn new() -> Self {
        LepccEncoder {
            section_size: 128,
            num_bytes_needed: 0,
            extent_3d: Extent3D::empty(),
            max_error: Point3D::origin(),
            cell_3d_vec: Vec::new(),
            y_delta_vec: Vec::new(),
            num_points_per_row_vec: Vec::new(),
            x_delta_vec: Vec::new(),
            z_vec: Vec::new(),
        }
    }

    pub fn compute_num_bytes_needed(
        &mut self,
        points: &[Point3D],
        max_x_err: f64,
        max_y_err: f64,
        max_z_err: f64,
    ) -> Result<i64> {
        self.num_bytes_needed = 0;

        if points.is_empty() || max_x_err <= 0.0 || max_y_err <= 0.0 || max_z_err <= 0.0 {
            return Err(LepccError::WrongParam("Invalid parameters".to_string()));
        }

        self.max_error = Point3D::new(max_x_err, max_y_err, max_z_err);
        self.extent_3d = Self::compute_3d_extent(points)?;

        self.quantize(points)?;
        self.convert_to_delta_model()?;

        let mut n_bytes = HEADER_SIZE as i64;

        n_bytes += self.compute_num_bytes_cut_in_segments(&self.y_delta_vec);
        n_bytes += self.compute_num_bytes_cut_in_segments(&self.num_points_per_row_vec);
        n_bytes += self.compute_num_bytes_cut_in_segments(&self.x_delta_vec);
        n_bytes += self.compute_num_bytes_cut_in_segments(&self.z_vec);

        self.num_bytes_needed = n_bytes;
        Ok(n_bytes)
    }

    pub fn encode(&self) -> Result<Vec<Byte>> {
        let mut buffer = Cursor::new(Vec::new());

        // Write TopHeader
        buffer.write_all(FILE_KEY)?;
        buffer.write_all(&K_CURR_VERSION.to_le_bytes())?;
        buffer.write_all(&0u32.to_le_bytes())?; // checksum (filled later)

        // Write Header1
        let blob_size_pos = buffer.position() as usize;
        buffer.write_all(&0i64.to_le_bytes())?; // blob_size placeholder

        // Write extent (lower and upper)
        self.write_point_3d(&mut buffer, &self.extent_3d.lower)?;
        self.write_point_3d(&mut buffer, &self.extent_3d.upper)?;

        // Write max error (offset 72)
        self.write_point_3d(&mut buffer, &self.max_error)?;

        // Write num points (offset 96)
        buffer.write_all(&(self.z_vec.len() as u32).to_le_bytes())?;

        // Write reserved (4 bytes, offset 100)
        buffer.write_all(&0u32.to_le_bytes())?;

        // Encode data segments
        self.encode_cut_in_segments(&mut buffer, &self.y_delta_vec, "y_delta_vec")?;
        self.encode_cut_in_segments(&mut buffer, &self.num_points_per_row_vec, "num_points_per_row_vec")?;
        self.encode_cut_in_segments(&mut buffer, &self.x_delta_vec, "x_delta_vec")?;
        self.encode_cut_in_segments(&mut buffer, &self.z_vec, "z_vec")?;

        // Update blob_size
        let mut result = buffer.into_inner();
        let blob_size = result.len() as i64;
        result[blob_size_pos..blob_size_pos + 8].copy_from_slice(&blob_size.to_le_bytes());

        // Compute and write checksum
        let checksum = compute_checksum_fletcher32(&result[16..blob_size as usize]);
        result[12..16].copy_from_slice(&checksum.to_le_bytes());

        if blob_size != self.num_bytes_needed {
            eprintln!(
                "Warning: blob_size {} != num_bytes_needed {}",
                blob_size, self.num_bytes_needed
            );
        }

        Ok(result)
    }

    fn compute_3d_extent(points: &[Point3D]) -> Result<Extent3D> {
        if points.is_empty() {
            return Ok(Extent3D::empty());
        }

        let mut ext = Extent3D {
            lower: points[0],
            upper: points[0],
        };

        for point in points {
            ext.lower.x = ext.lower.x.min(point.x);
            ext.lower.y = ext.lower.y.min(point.y);
            ext.lower.z = ext.lower.z.min(point.z);
            ext.upper.x = ext.upper.x.max(point.x);
            ext.upper.y = ext.upper.y.max(point.y);
            ext.upper.z = ext.upper.z.max(point.z);
        }

        Ok(ext)
    }

    fn quantize(&mut self, points: &[Point3D]) -> Result<()> {
        // Debug output (can be enabled via environment variable)
        let debug = std::env::var("LEPCC_DEBUG").is_ok();

        let cell_size_x = 2.0 * self.max_error.x;
        let cell_size_y = 2.0 * self.max_error.y;
        let cell_size_z = 2.0 * self.max_error.z;

        if debug {
            eprintln!("=== Quantize Debug ===");
            eprintln!("Input points: {}", points.len());
            eprintln!(
                "max_error: ({}, {}, {})",
                self.max_error.x, self.max_error.y, self.max_error.z
            );
            eprintln!(
                "extent: min=({:.6}, {:.6}, {:.6}), max=({:.6}, {:.6}, {:.6})",
                self.extent_3d.lower.x,
                self.extent_3d.lower.y,
                self.extent_3d.lower.z,
                self.extent_3d.upper.x,
                self.extent_3d.upper.y,
                self.extent_3d.upper.z
            );
            eprintln!(
                "cell_size: ({:.6}, {:.6}, {:.6})",
                cell_size_x, cell_size_y, cell_size_z
            );
        }

        let nx = ((self.extent_3d.upper.x - self.extent_3d.lower.x) / cell_size_x + 0.5) as i64 + 1;
        let ny = ((self.extent_3d.upper.y - self.extent_3d.lower.y) / cell_size_y + 0.5) as i64 + 1;
        let nz = ((self.extent_3d.upper.z - self.extent_3d.lower.z) / cell_size_z + 0.5) as i64 + 1;

        if debug {
            eprintln!("grid size: ({}, {}, {})", nx, ny, nz);
        }

        if nx <= 0
            || ny <= 0
            || nz <= 0
            || nx > i32::MAX as i64
            || ny > i32::MAX as i64
            || nz > i32::MAX as i64
        {
            return Err(LepccError::QuantizeVirtualRasterTooBig);
        }

        let nx = nx as i32;
        let ny = ny as i32;
        let nz = nz as i32;

        self.cell_3d_vec.clear();
        self.cell_3d_vec.reserve(points.len());

        let p0 = self.extent_3d.lower;

        for (i, point) in points.iter().enumerate() {
            let ix = ((point.x - p0.x) / cell_size_x + 0.5) as i32;
            let iy = ((point.y - p0.y) / cell_size_y + 0.5) as i32;
            let iz = ((point.z - p0.z) / cell_size_z + 0.5) as i32;

            // Match C++ bounds check: only check upper bounds
            if ix >= nx || iy >= ny || iz >= nz {
                return Err(LepccError::QuantizeIndexOutOfRange {
                    index: iz as i64,
                    limit: nz as usize,
                });
            }

            let cell = Cell3D::new(ix, iy, iz, i, nx);
            self.cell_3d_vec.push(cell);
        }

        if debug {
            // Print first 5 cells
            eprintln!("First 5 quantized cells:");
            for (i, cell) in self.cell_3d_vec.iter().enumerate().take(5) {
                eprintln!(
                    "  [{}]: x={}, y={}, z={}, orig_index={}, xyCellIndex={}",
                    i, cell.x, cell.y, cell.z, cell.orig_pt_index, cell.xy_cell_index
                );
            }

            // Print cell ranges
            let x_range = (
                self.cell_3d_vec.iter().map(|c| c.x).min().unwrap_or(0),
                self.cell_3d_vec.iter().map(|c| c.x).max().unwrap_or(0),
            );
            let y_range = (
                self.cell_3d_vec.iter().map(|c| c.y).min().unwrap_or(0),
                self.cell_3d_vec.iter().map(|c| c.y).max().unwrap_or(0),
            );
            let z_range = (
                self.cell_3d_vec.iter().map(|c| c.z).min().unwrap_or(0),
                self.cell_3d_vec.iter().map(|c| c.z).max().unwrap_or(0),
            );

            eprintln!(
                "Cell ranges: X=[{}, {}], Y=[{}, {}], Z=[{}, {}]",
                x_range.0, x_range.1, y_range.0, y_range.1, z_range.0, z_range.1
            );

            // Check unique y indices
            let unique_y: std::collections::HashSet<_> =
                self.cell_3d_vec.iter().map(|c| c.y).collect();
            eprintln!("Unique y indices: {}", unique_y.len());
            if unique_y.len() <= 2 {
                eprintln!("  Warning: Only {} unique y indices!", unique_y.len());
                for &y in &unique_y {
                    let count = self.cell_3d_vec.iter().filter(|c| c.y == y).count();
                    eprintln!("    y={}: {} points", y, count);
                }
            }
        }

        Ok(())
    }

    fn convert_to_delta_model(&mut self) -> Result<()> {
        let debug = std::env::var("LEPCC_DEBUG").is_ok();

        if debug {
            eprintln!("=== ConvertToDeltaModel Debug ===");
            eprintln!("Input cells: {}", self.cell_3d_vec.len());
        }

        if self.cell_3d_vec.is_empty() {
            return Err(LepccError::Failed("No points to convert".to_string()));
        }

        let num_points = self.cell_3d_vec.len();

        // Sort by xyCellIndex
        self.cell_3d_vec.sort_by(cell3d_compare);

        if debug {
            eprintln!("After sorting, first 5 cells:");
            for (i, cell) in self.cell_3d_vec.iter().enumerate().take(5) {
                eprintln!(
                    "  [{}]: x={}, y={}, z={}, xyCellIndex={}",
                    i, cell.x, cell.y, cell.z, cell.xy_cell_index
                );
            }
        }

        // Clear output vectors
        self.y_delta_vec.clear();
        self.num_points_per_row_vec.clear();
        self.x_delta_vec.clear();
        self.z_vec.clear();

        // Process rows
        let mut n_pts_per_row: u32 = 0;
        let mut prev_row: i32 = 0;
        let mut y_curr = self.cell_3d_vec[0].y;

        for i in 0..num_points {
            let iy = self.cell_3d_vec[i].y;

            if iy == y_curr {
                n_pts_per_row += 1;
            } else {
                self.y_delta_vec.push((y_curr - prev_row) as u32);
                self.num_points_per_row_vec.push(n_pts_per_row);

                n_pts_per_row = 1;
                prev_row = y_curr;
                y_curr = iy;
            }
        }

        // Push the last row
        self.y_delta_vec.push((y_curr - prev_row) as u32);
        self.num_points_per_row_vec.push(n_pts_per_row);

        if debug {
            eprintln!("y_delta_vec ({} elements):", self.y_delta_vec.len());
            for (i, &val) in self.y_delta_vec.iter().enumerate() {
                if i < 10 {
                    eprintln!("  [{}]: {}", i, val);
                } else if i == 10 {
                    eprintln!("  ... ({} elements total)", self.y_delta_vec.len());
                    break;
                }
            }

            eprintln!(
                "num_points_per_row_vec ({} elements):",
                self.num_points_per_row_vec.len()
            );
            for (i, &val) in self.num_points_per_row_vec.iter().enumerate() {
                if i < 10 {
                    eprintln!("  [{}]: {}", i, val);
                } else if i == 10 {
                    eprintln!("  ... ({} elements total)", self.num_points_per_row_vec.len());
                    break;
                }
            }

            let non_zero_y = self.y_delta_vec.iter().filter(|&&x| x != 0).count();
            eprintln!("Non-zero elements in y_delta_vec: {}/{}", non_zero_y, self.y_delta_vec.len());
        }

        // Process x and z for each row
        let num_occupied_rows = self.y_delta_vec.len();
        let mut iy: i32 = 0;
        let mut cnt: usize = 0;

        for i in 0..num_occupied_rows {
            iy += self.y_delta_vec[i] as i32;
            let mut prev_col: i32 = 0;

            for _j in 0..self.num_points_per_row_vec[i] as usize {
                let pt = &self.cell_3d_vec[cnt];
                cnt += 1;

                if pt.y != iy {
                    return Err(LepccError::Failed(
                        "Point y coordinate mismatch".to_string(),
                    ));
                }

                let x_delta = pt.x - prev_col;
                self.x_delta_vec.push(x_delta as u32);
                prev_col = pt.x;

                self.z_vec.push(pt.z as u32);
            }
        }

        if debug {
            eprintln!(
                "x_delta_vec ({} elements):",
                self.x_delta_vec.len()
            );
            eprintln!("First 10:");
            for i in 0..std::cmp::min(10, self.x_delta_vec.len()) {
                eprintln!("  [{}]: {}", i, self.x_delta_vec[i]);
            }

            eprintln!("z_vec ({} elements):", self.z_vec.len());
            eprintln!("First 10:");
            for i in 0..std::cmp::min(10, self.z_vec.len()) {
                eprintln!("  [{}]: {}", i, self.z_vec[i]);
            }

            let non_zero_x = self.x_delta_vec.iter().filter(|&&x| x != 0).count();
            let non_zero_z = self.z_vec.iter().filter(|&&x| x != 0).count();

            eprintln!("Non-zero elements:");
            eprintln!("  x_delta_vec: {}/{}", non_zero_x, self.x_delta_vec.len());
            eprintln!("  z_vec: {}/{}", non_zero_z, self.z_vec.len());
        }

        Ok(())
    }

    fn compute_num_bytes_cut_in_segments(&self, data_vec: &[u32]) -> i64 {
        let num_sections =
            (data_vec.len() + (self.section_size as usize - 1)) / self.section_size as usize;
        let len_last_section = data_vec.len() - (num_sections - 1) * self.section_size as usize;

        let mut section_min_vec: Vec<u32> = Vec::with_capacity(num_sections);
        let mut n_bytes = 0i32;

        for i in 0..num_sections {
            let len = if i < num_sections - 1 {
                self.section_size as usize
            } else {
                len_last_section
            };

            let start = i * self.section_size as usize;
            let end = start + len;

            let mut min_elem = data_vec[start];
            let mut max_elem = min_elem;

            for j in start..end {
                min_elem = min_elem.min(data_vec[j]);
                max_elem = max_elem.max(data_vec[j]);
            }

            section_min_vec.push(min_elem);

            let range = max_elem - min_elem;
            n_bytes += BitStuffer2::compute_num_bytes_needed_simple(len as u32, range) as i32;
        }

        let max_of_section_mins = *section_min_vec.iter().max().unwrap_or(&0);
        n_bytes +=
            BitStuffer2::compute_num_bytes_needed_simple(num_sections as u32, max_of_section_mins)
                as i32;

        n_bytes as i64
    }

    fn encode_cut_in_segments(
        &self,
        buffer: &mut Cursor<Vec<Byte>>,
        data_vec: &[u32],
        name: &str,
    ) -> Result<()> {
        let debug = std::env::var("LEPCC_DEBUG").is_ok();

        if debug {
            eprintln!("=== EncodeCutInSegments Debug ({}) ===", name);
            eprintln!("Input length: {}", data_vec.len());
            eprintln!("Section size: {}", self.section_size);

            // Print first 10 elements
            eprintln!("First 10 elements:");
            for (i, &val) in data_vec.iter().take(10).enumerate() {
                eprintln!("  [{}]: {}", i, val);
            }
            if data_vec.len() > 10 {
                eprintln!("  ... ({} elements total)", data_vec.len());
            }
        }

        let num_sections =
            (data_vec.len() + (self.section_size as usize - 1)) / self.section_size as usize;

        if debug {
            eprintln!("Number of sections: {}", num_sections);
        }

        let mut section_min_vec = Vec::new();

        for i in 0..num_sections {
            let start = i * self.section_size as usize;
            let end = std::cmp::min(start + self.section_size as usize, data_vec.len());

            let min_elem = data_vec[start..end].iter().min().copied().unwrap_or(0);
            let max_elem = data_vec[start..end].iter().max().copied().unwrap_or(0);

            section_min_vec.push(min_elem);

            if debug {
                eprintln!(
                    "  Section [{}]: len={}, min={}, max={}, range={}",
                    i,
                    end - start,
                    min_elem,
                    max_elem,
                    max_elem - min_elem
                );
            }
        }

        let max_of_section_mins = *section_min_vec.iter().max().unwrap_or(&0);
        let min_of_section_mins = *section_min_vec.iter().min().unwrap_or(&u32::MAX);

        if debug {
            eprintln!(
                "Section mins range: [{}, {}]",
                min_of_section_mins, max_of_section_mins
            );
            eprintln!("Encoding section mins ({} elements)...", section_min_vec.len());
        }

        // Write section mins
        let encoded_mins = BitStuffer2::encode_simple(&section_min_vec)?;
        if debug {
            eprintln!("  Encoded mins size: {} bytes", encoded_mins.len());
            eprintln!("  First 10 bytes: {:02X?}", encoded_mins.iter().take(10).collect::<Vec<_>>());
        }
        buffer.write_all(&encoded_mins)?;

        // Write sections
        for i in 0..num_sections {
            let start = i * self.section_size as usize;
            let end = std::cmp::min(start + self.section_size as usize, data_vec.len());
            let min_elem = section_min_vec[i];

            let mut zero_based_data: Vec<u32> = Vec::new();
            for j in start..end {
                zero_based_data.push(data_vec[j] - min_elem);
            }

            if debug {
                eprintln!("  Encoding section [{}] ({} elements)...", i, zero_based_data.len());
            }

            let encoded = BitStuffer2::encode_simple(&zero_based_data)?;
            if debug {
                eprintln!("    Encoded size: {} bytes", encoded.len());
            }
            buffer.write_all(&encoded)?;
        }

        Ok(())
    }

    fn write_point_3d(&self, buffer: &mut Cursor<Vec<Byte>>, point: &Point3D) -> Result<()> {
        buffer.write_all(&point.x.to_le_bytes())?;
        buffer.write_all(&point.y.to_le_bytes())?;
        buffer.write_all(&point.z.to_le_bytes())?;
        Ok(())
    }
}

pub struct LepccDecoder {
    section_size: i32,
    decode_size: usize,
}

impl Default for LepccDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl LepccDecoder {
    pub fn new() -> Self {
        LepccDecoder {
            section_size: 128,
            decode_size: 0,
        }
    }

    pub fn get_decode_size(&self) -> usize {
        self.decode_size
    }

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

        let blob_size = i64::from_le_bytes([
            data[16], data[17], data[18], data[19], data[20], data[21], data[22], data[23],
        ]);
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

        let num_points = u32::from_le_bytes([data[96], data[97], data[98], data[99]]);
        Ok(num_points)
    }

    pub fn decode(data: &[Byte]) -> Result<Vec<Point3D>> {
        let debug = std::env::var("LEPCC_DEBUG").is_ok();

        let blob_size = Self::get_blob_size(data)? as usize;
        if data.len() < blob_size {
            return Err(LepccError::BufferTooSmall {
                needed: blob_size,
                provided: data.len(),
            });
        }

        if data.len() < HEADER_SIZE {
            return Err(LepccError::BufferTooSmall {
                needed: HEADER_SIZE,
                provided: data.len(),
            });
        }

        // Read header
        if &data[0..10] != FILE_KEY {
            return Err(LepccError::NotLepcc("Invalid file key".to_string()));
        }

        _ = u16::from_le_bytes([data[10], data[11]]); // version

        let header_offset = 16;

        // Read extent
        let x_min = f64::from_le_bytes([
            data[header_offset + 8],
            data[header_offset + 9],
            data[header_offset + 10],
            data[header_offset + 11],
            data[header_offset + 12],
            data[header_offset + 13],
            data[header_offset + 14],
            data[header_offset + 15],
        ]);
        let y_min = f64::from_le_bytes([
            data[header_offset + 16],
            data[header_offset + 17],
            data[header_offset + 18],
            data[header_offset + 19],
            data[header_offset + 20],
            data[header_offset + 21],
            data[header_offset + 22],
            data[header_offset + 23],
        ]);
        let z_min = f64::from_le_bytes([
            data[header_offset + 24],
            data[header_offset + 25],
            data[header_offset + 26],
            data[header_offset + 27],
            data[header_offset + 28],
            data[header_offset + 29],
            data[header_offset + 30],
            data[header_offset + 31],
        ]);

        let x_max = f64::from_le_bytes([
            data[header_offset + 32],
            data[header_offset + 33],
            data[header_offset + 34],
            data[header_offset + 35],
            data[header_offset + 36],
            data[header_offset + 37],
            data[header_offset + 38],
            data[header_offset + 39],
        ]);
        let y_max = f64::from_le_bytes([
            data[header_offset + 40],
            data[header_offset + 41],
            data[header_offset + 42],
            data[header_offset + 43],
            data[header_offset + 44],
            data[header_offset + 45],
            data[header_offset + 46],
            data[header_offset + 47],
        ]);
        let z_max = f64::from_le_bytes([
            data[header_offset + 48],
            data[header_offset + 49],
            data[header_offset + 50],
            data[header_offset + 51],
            data[header_offset + 52],
            data[header_offset + 53],
            data[header_offset + 54],
            data[header_offset + 55],
        ]);

        let extent = Extent3D {
            lower: Point3D::new(x_min, y_min, z_min),
            upper: Point3D::new(x_max, y_max, z_max),
        };

        // Read max error (offset 72 after blobSize+extent)
        let max_err_x = f64::from_le_bytes([
            data[header_offset + 56],
            data[header_offset + 57],
            data[header_offset + 58],
            data[header_offset + 59],
            data[header_offset + 60],
            data[header_offset + 61],
            data[header_offset + 62],
            data[header_offset + 63],
        ]);
        let max_err_y = f64::from_le_bytes([
            data[header_offset + 64],
            data[header_offset + 65],
            data[header_offset + 66],
            data[header_offset + 67],
            data[header_offset + 68],
            data[header_offset + 69],
            data[header_offset + 70],
            data[header_offset + 71],
        ]);
        let max_err_z = f64::from_le_bytes([
            data[header_offset + 72],
            data[header_offset + 73],
            data[header_offset + 74],
            data[header_offset + 75],
            data[header_offset + 76],
            data[header_offset + 77],
            data[header_offset + 78],
            data[header_offset + 79],
        ]);

        // Read num points (offset 80 after blobSize+extent+maxError)
        let num_points = u32::from_le_bytes([
            data[header_offset + 80],
            data[header_offset + 81],
            data[header_offset + 82],
            data[header_offset + 83],
        ]);

        // Decode data segments
        let data_start = HEADER_SIZE;
        let mut decoder = Self::new();

        let y_delta_vec = decoder.decode_cut_in_segments(&data[data_start..])?;
        let offset_after_y = data_start + decoder.decode_size;

        if debug {
            eprintln!("=== Decode Debug ===");
            eprintln!("y_delta_vec: {} elements", y_delta_vec.len());
            eprintln!("  Values: {:?}", y_delta_vec);
            eprintln!("  Sum of num_points_per_row should be: {}", y_delta_vec.iter().sum::<u32>());
        }

        let num_points_per_row_vec = decoder.decode_cut_in_segments(&data[offset_after_y..])?;
        let offset_after_row = offset_after_y + decoder.decode_size;

        if debug {
            eprintln!("num_points_per_row_vec: {} elements", num_points_per_row_vec.len());
            eprintln!("  Values: {:?}", num_points_per_row_vec.iter().take(10).collect::<Vec<_>>());
        }

        let x_delta_vec = decoder.decode_cut_in_segments(&data[offset_after_row..])?;
        let offset_after_x = offset_after_row + decoder.decode_size;

        if debug {
            eprintln!("x_delta_vec: {} elements", x_delta_vec.len());
            eprintln!("  First 10: {:?}", x_delta_vec.iter().take(10).collect::<Vec<_>>());
        }

        let z_vec = decoder.decode_cut_in_segments(&data[offset_after_x..])?;

        if debug {
            eprintln!("z_vec: {} elements", z_vec.len());
            eprintln!("  First 10: {:?}", z_vec.iter().take(10).collect::<Vec<_>>());
        }

        // Reconstruct points
        let mut points = Vec::with_capacity(num_points as usize);

        let p0 = extent.lower;
        let p1 = extent.upper;
        let cw = Point3D::new(2.0 * max_err_x, 2.0 * max_err_y, 2.0 * max_err_z);

        let mut iy: i32 = 0;
        let n_rows = y_delta_vec.len();
        let mut cnt: usize = 0;

        for i in 0..n_rows {
            iy += y_delta_vec[i] as i32;
            let mut ix: i32 = 0;
            let n_pts = num_points_per_row_vec[i] as usize;

            for _ in 0..n_pts {
                if cnt >= x_delta_vec.len() || cnt >= z_vec.len() {
                    if debug {
                        eprintln!("ERROR: cnt {} exceeds x_delta_vec.len()={} or z_vec.len()={}",
                            cnt, x_delta_vec.len(), z_vec.len());
                    }
                    break;
                }
                ix += x_delta_vec[cnt] as i32;
                let iz = z_vec[cnt] as i32;
                cnt += 1;

                let x = p0.x + (ix as f64) * cw.x;
                let y = p0.y + (iy as f64) * cw.y;
                let z = p0.z + (iz as f64) * cw.z;

                // Clamp to extent
                let x = x.min(p1.x);
                let y = y.min(p1.y);
                let z = z.min(p1.z);

                points.push(Point3D::new(x, y, z));
            }
        }

        if debug {
            eprintln!("Total points decoded: {}", points.len());
        }

        Ok(points)
    }

    fn decode_cut_in_segments(&mut self, data: &[Byte]) -> Result<Vec<u32>> {
        let debug = std::env::var("LEPCC_DEBUG").is_ok();
        self.decode_size = 0;
        let mut pos = 0;

        if data.is_empty() {
            return Ok(Vec::new());
        }

        if debug {
            eprintln!("=== decode_cut_in_segments Debug ===");
            eprintln!("Total input: {} bytes", data.len());
        }

        // Decode section mins
        let section_min_vec = BitStuffer2::decode(&data[pos..])?;
        // Calculate bytes consumed by section mins
        let section_mins_header = data[pos];
        let section_mins_size_mode = (section_mins_header >> 6) & 0x3;
        let section_mins_elem_size: usize = match section_mins_size_mode {
            0 => 4,
            1 => 2,
            2 => 1,
            _ => 4,
        };
        let section_mins_num_bits = (section_mins_header & 0x1F) as usize;
        let section_mins_num_elem: usize = if section_mins_size_mode == 2 {
            data[pos + 1] as usize
        } else if section_mins_size_mode == 1 {
            u16::from_le_bytes([data[pos + 1], data[pos + 2]]) as usize
        } else {
            u32::from_le_bytes([data[pos + 1], data[pos + 2], data[pos + 3], data[pos + 4]]) as usize
        };
        let section_mins_data_size = (section_mins_num_elem * section_mins_num_bits + 7) / 8;
        let section_mins_total = 1 + section_mins_elem_size + section_mins_data_size;

        if debug {
            eprintln!("Section mins: {} elements, {} bytes total", section_mins_num_elem, section_mins_total);
            eprintln!("  Values: {:?}", section_min_vec);
        }

        pos += section_mins_total;

        let num_sections = section_min_vec.len();
        let mut result = Vec::with_capacity(num_sections * self.section_size as usize);

        // Decode each section
        for i in 0..num_sections {
            if pos >= data.len() {
                return Err(LepccError::BufferTooSmall {
                    needed: pos + 1,
                    provided: data.len(),
                });
            }

            let section_data = BitStuffer2::decode(&data[pos..])?;
            let min_elem = section_min_vec[i];

            if debug && i < 3 {
                eprintln!("  Section [{}]: min_elem={}, decoded={:?}", i, min_elem,
                    section_data.iter().take(10).collect::<Vec<_>>());
            }

            // Calculate bytes consumed by section data
            let section_header = data[pos];
            let section_size_mode = (section_header >> 6) & 0x3;
            let section_elem_size: usize = match section_size_mode {
                0 => 4,
                1 => 2,
                2 => 1,
                _ => 4,
            };
            let section_num_bits = (section_header & 0x1F) as usize;
            let section_num_elem: usize = if section_size_mode == 2 {
                data[pos + 1] as usize
            } else if section_size_mode == 1 {
                u16::from_le_bytes([data[pos + 1], data[pos + 2]]) as usize
            } else {
                u32::from_le_bytes([data[pos + 1], data[pos + 2], data[pos + 3], data[pos + 4]]) as usize
            };
            let section_data_size = (section_num_elem * section_num_bits + 7) / 8;
            let section_total = 1 + section_elem_size + section_data_size;

            if debug && i < 3 {
                eprintln!("  Total: {} bytes (header={}, data={}, elem_size={}, num_elem={}, num_bits={})",
                    section_total, section_elem_size, section_data_size, section_elem_size, section_num_elem, section_num_bits);
            }

            pos += section_total;

            // Add min_elem back to get the actual values
            for val in section_data {
                result.push(val + min_elem);
            }
        }

        if debug {
            eprintln!("Total decoded: {} elements, {} bytes consumed", result.len(), pos);
        }

        self.decode_size = pos;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_extent() {
        let points = vec![
            Point3D::new(0.0, 0.0, 0.0),
            Point3D::new(1.0, 1.0, 1.0),
            Point3D::new(2.0, 0.0, 1.0),
        ];

        let ext = LepccEncoder::compute_3d_extent(&points).unwrap();
        assert_eq!(ext.lower, Point3D::new(0.0, 0.0, 0.0));
        assert_eq!(ext.upper, Point3D::new(2.0, 1.0, 1.0));
    }

    #[test]
    fn test_encode_decode_header() {
        let points = vec![
            Point3D::new(0.0, 0.0, 0.0),
            Point3D::new(1.0, 1.0, 1.0),
            Point3D::new(2.0, 0.0, 1.0),
        ];

        let mut encoder = LepccEncoder::new();
        let _size = encoder
            .compute_num_bytes_needed(&points, 0.01, 0.01, 0.01)
            .unwrap();
        let encoded = encoder.encode().unwrap();

        let blob_size = LepccDecoder::get_blob_size(&encoded).unwrap();
        assert_eq!(blob_size, encoded.len() as u32);

        let num_points = LepccDecoder::get_num_points(&encoded).unwrap();
        assert_eq!(num_points, points.len() as u32);
    }

    #[test]
    fn test_cell3d_sorting() {
        let mut cells = vec![
            Cell3D::new(1, 2, 0, 0, 10),
            Cell3D::new(0, 1, 0, 1, 10),
            Cell3D::new(2, 1, 0, 2, 10),
        ];

        cells.sort_by(cell3d_compare);

        assert_eq!(cells[0].y, 1);
        assert_eq!(cells[1].y, 1);
        assert_eq!(cells[2].y, 2);
        assert_eq!(cells[0].x, 0);
        assert_eq!(cells[1].x, 2);
    }
}
