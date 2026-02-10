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

//! High-level API for LEPCC compression and decompression
//!
//! This module provides convenient functions for compressing and decompressing
//! point cloud data that can be used by other Rust projects.

use crate::cluster_rgb::{ClusterRgbDecoder, ClusterRgbEncoder};
use crate::error::{LepccError, Result};
use crate::intensity::{IntensityDecoder, IntensityEncoder};
use crate::lepcc_xyz::{LepccDecoder, LepccEncoder};
use crate::types::{Point3D, RGB, BlobType};

/// Compress XYZ coordinates from a vertex array
///
/// # Arguments
///
/// * `vertices` - Slice of 3D points to compress
/// * `max_x_err` - Maximum compression error tolerated for X coordinates
/// * `max_y_err` - Maximum compression error tolerated for Y coordinates
/// * `max_z_err` - Maximum compression error tolerated for Z coordinates
///
/// # Returns
///
/// Compressed binary data compatible with I3S .pccxyz format
///
/// # Example
///
/// ```ignore
/// use lepcc::api::compress_xyz;
/// use lepcc::types::Point3D;
///
/// let vertices = vec![
///     Point3D::new(0.0, 0.0, 0.0),
///     Point3D::new(1.0, 1.0, 1.0),
///     Point3D::new(2.0, 0.0, 1.0),
/// ];
///
/// let compressed = compress_xyz(&vertices, 0.01, 0.01, 0.01)?;
/// ```
pub fn compress_xyz(vertices: &[Point3D], max_x_err: f64, max_y_err: f64, max_z_err: f64) -> Result<Vec<u8>> {
    if vertices.is_empty() {
        return Err(LepccError::WrongParam("No vertices provided".to_string()));
    }

    let mut encoder = LepccEncoder::new();
    encoder.compute_num_bytes_needed(vertices, max_x_err, max_y_err, max_z_err)?;
    encoder.encode()
}

/// Compress XYZ coordinates from a flat f64 array
///
/// This is a convenience function for cases where vertices are stored as a
/// flat array: [x0, y0, z0, x1, y1, z1, ...]
///
/// # Arguments
///
/// * `xyz_array` - Flat array of XYZ coordinates (length must be multiple of 3)
/// * `max_x_err` - Maximum compression error tolerated for X coordinates
/// * `max_y_err` - Maximum compression error tolerated for Y coordinates
/// * `max_z_err` - Maximum compression error tolerated for Z coordinates
///
/// # Returns
///
/// Compressed binary data compatible with I3S .pccxyz format
///
/// # Example
///
/// ```ignore
/// use lepcc::api::compress_xyz_array;
///
/// let xyz_array = vec![
///     0.0, 0.0, 0.0,   // Point 0
///     1.0, 1.0, 1.0,   // Point 1
///     2.0, 0.0, 1.0,   // Point 2
/// ];
///
/// let compressed = compress_xyz_array(&xyz_array, 0.01, 0.01, 0.01)?;
/// ```
pub fn compress_xyz_array(xyz_array: &[f64], max_x_err: f64, max_y_err: f64, max_z_err: f64) -> Result<Vec<u8>> {
    if xyz_array.is_empty() {
        return Err(LepccError::WrongParam("No coordinates provided".to_string()));
    }

    if xyz_array.len() % 3 != 0 {
        return Err(LepccError::WrongParam(
            "XYZ array length must be a multiple of 3".to_string(),
        ));
    }

    let vertices: Vec<Point3D> = xyz_array
        .chunks_exact(3)
        .map(|chunk| Point3D::new(chunk[0], chunk[1], chunk[2]))
        .collect();

    compress_xyz(&vertices, max_x_err, max_y_err, max_z_err)
}

/// Decompress XYZ coordinates from binary data
///
/// # Arguments
///
/// * `data` - Compressed binary data (e.g., from .pccxyz file)
///
/// # Returns
///
/// Vector of 3D points
///
/// # Example
///
/// ```ignore
/// use lepcc::api::{compress_xyz, decompress_xyz};
///
/// let vertices = vec![
///     Point3D::new(0.0, 0.0, 0.0),
///     Point3D::new(1.0, 1.0, 1.0),
/// ];
///
/// let compressed = compress_xyz(&vertices, 0.01, 0.01, 0.01)?;
/// let decompressed = decompress_xyz(&compressed)?;
/// ```
pub fn decompress_xyz(data: &[u8]) -> Result<Vec<Point3D>> {
    LepccDecoder::decode(data)
}

/// Decompress XYZ coordinates to a flat f64 array
///
/// This is a convenience function that returns decompressed data as a flat
/// array: [x0, y0, z0, x1, y1, z1, ...]
///
/// # Arguments
///
/// * `data` - Compressed binary data (e.g., from .pccxyz file)
///
/// # Returns
///
/// Flat array of XYZ coordinates
///
/// # Example
///
/// ```ignore
/// use lepcc::api::{compress_xyz_array, decompress_xyz_array};
///
/// let xyz_array = vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
/// let compressed = compress_xyz_array(&xyz_array, 0.01, 0.01, 0.01)?;
/// let decompressed = decompress_xyz_array(&compressed)?;
/// ```
pub fn decompress_xyz_array(data: &[u8]) -> Result<Vec<f64>> {
    let points = decompress_xyz(data)?;
    let mut result = Vec::with_capacity(points.len() * 3);

    for point in points {
        result.push(point.x);
        result.push(point.y);
        result.push(point.z);
    }

    Ok(result)
}

/// Compress RGB colors from a color array
///
/// # Arguments
///
/// * `colors` - Slice of RGB colors to compress
///
/// # Returns
///
/// Compressed binary data compatible with I3S .pccrgb format
///
/// # Example
///
/// ```ignore
/// use lepcc::api::compress_rgb;
/// use lepcc::types::RGB;
///
/// let colors = vec![
///     RGB { r: 255, g: 0, b: 0 },
///     RGB { r: 0, g: 255, b: 0 },
///     RGB { r: 0, g: 0, b: 255 },
/// ];
///
/// let compressed = compress_rgb(&colors)?;
/// ```
pub fn compress_rgb(colors: &[RGB]) -> Result<Vec<u8>> {
    if colors.is_empty() {
        return Err(LepccError::WrongParam("No colors provided".to_string()));
    }

    let mut encoder = ClusterRgbEncoder::new();
    encoder.compute_num_bytes_needed(colors)?;
    encoder.encode()
}

/// Compress RGB colors from a flat u8 array
///
/// This is a convenience function for cases where colors are stored as a
/// flat array: [r0, g0, b0, r1, g1, b1, ...]
///
/// # Arguments
///
/// * `rgb_array` - Flat array of RGB values (length must be multiple of 3)
///
/// # Returns
///
/// Compressed binary data compatible with I3S .pccrgb format
///
/// # Example
///
/// ```ignore
/// use lepcc::api::compress_rgb_array;
///
/// let rgb_array = vec![
///     255, 0, 0,   // Red
///     0, 255, 0,   // Green
///     0, 0, 255,   // Blue
/// ];
///
/// let compressed = compress_rgb_array(&rgb_array)?;
/// ```
pub fn compress_rgb_array(rgb_array: &[u8]) -> Result<Vec<u8>> {
    if rgb_array.is_empty() {
        return Err(LepccError::WrongParam("No colors provided".to_string()));
    }

    if rgb_array.len() % 3 != 0 {
        return Err(LepccError::WrongParam(
            "RGB array length must be a multiple of 3".to_string(),
        ));
    }

    let colors: Vec<RGB> = rgb_array
        .chunks_exact(3)
        .map(|chunk| RGB {
            r: chunk[0],
            g: chunk[1],
            b: chunk[2],
        })
        .collect();

    compress_rgb(&colors)
}

/// Decompress RGB colors from binary data
///
/// # Arguments
///
/// * `data` - Compressed binary data (e.g., from .pccrgb file)
///
/// # Returns
///
/// Vector of RGB colors
///
/// # Example
///
/// ```ignore
/// use lepcc::api::{compress_rgb, decompress_rgb};
/// use lepcc::types::RGB;
///
/// let colors = vec![
///     RGB { r: 255, g: 0, b: 0 },
///     RGB { r: 0, g: 255, b: 0 },
/// ];
///
/// let compressed = compress_rgb(&colors)?;
/// let decompressed = decompress_rgb(&compressed)?;
/// ```
pub fn decompress_rgb(data: &[u8]) -> Result<Vec<RGB>> {
    ClusterRgbDecoder::decode(data)
}

/// Decompress RGB colors to a flat u8 array
///
/// This is a convenience function that returns decompressed data as a flat
/// array: [r0, g0, b0, r1, g1, b1, ...]
///
/// # Arguments
///
/// * `data` - Compressed binary data (e.g., from .pccrgb file)
///
/// # Returns
///
/// Flat array of RGB values
///
/// # Example
///
/// ```ignore
/// use lepcc::api::{compress_rgb_array, decompress_rgb_array};
///
/// let rgb_array = vec![255, 0, 0, 0, 255, 0];
/// let compressed = compress_rgb_array(&rgb_array)?;
/// let decompressed = decompress_rgb_array(&compressed)?;
/// ```
pub fn decompress_rgb_array(data: &[u8]) -> Result<Vec<u8>> {
    let colors = decompress_rgb(data)?;
    let mut result = Vec::with_capacity(colors.len() * 3);

    for color in colors {
        result.push(color.r);
        result.push(color.g);
        result.push(color.b);
    }

    Ok(result)
}

/// Compress intensity values
///
/// # Arguments
///
/// * `intensities` - Slice of intensity values (16-bit)
///
/// # Returns
///
/// Compressed binary data
///
/// # Example
///
/// ```ignore
/// use lepcc::api::{compress_intensity, decompress_intensity};
///
/// let intensities = vec![100u16, 200, 300, 150];
/// let compressed = compress_intensity(&intensities)?;
/// let decompressed = decompress_intensity(&compressed)?;
/// ```
pub fn compress_intensity(intensities: &[u16]) -> Result<Vec<u8>> {
    if intensities.is_empty() {
        return Err(LepccError::WrongParam("No intensities provided".to_string()));
    }

    let mut encoder = IntensityEncoder::new();
    encoder.compute_num_bytes_needed(intensities)?;
    encoder.encode(intensities)
}

/// Decompress intensity values
///
/// # Arguments
///
/// * `data` - Compressed binary data
///
/// # Returns
///
/// Vector of intensity values
pub fn decompress_intensity(data: &[u8]) -> Result<Vec<u16>> {
    IntensityDecoder::decode(data)
}

/// Compress flag bytes (e.g., classification, return type)
///
/// # Arguments
///
/// * `flag_bytes` - Slice of byte values
///
/// # Returns
///
/// Compressed binary data
///
/// # Example
///
/// ```ignore
/// use lepcc::api::{compress_flag_bytes, decompress_flag_bytes};
///
/// let flags = vec![1u8, 2, 3, 1, 2, 3];
/// let compressed = compress_flag_bytes(&flags)?;
/// let decompressed = decompress_flag_bytes(&compressed)?;
/// ```
pub fn compress_flag_bytes(flag_bytes: &[u8]) -> Result<Vec<u8>> {
    if flag_bytes.is_empty() {
        return Err(LepccError::WrongParam("No flag bytes provided".to_string()));
    }

    let mut encoder = super::flag_bytes::FlagBytesEncoder::new();
    encoder.compute_num_bytes_needed(flag_bytes)?;
    encoder.encode(flag_bytes)
}

/// Decompress flag bytes
///
/// # Arguments
///
/// * `data` - Compressed binary data
///
/// # Returns
///
/// Vector of byte values
pub fn decompress_flag_bytes(data: &[u8]) -> Result<Vec<u8>> {
    super::flag_bytes::FlagBytesDecoder::decode(data)
}

/// Get the blob type from compressed data
///
/// # Arguments
///
/// * `data` - Compressed binary data
///
/// # Returns
///
/// The blob type (XYZ, RGB, Intensity, or FlagBytes)
pub fn get_blob_type(data: &[u8]) -> Result<BlobType> {
    // Check XYZ
    if data.len() >= 10 && &data[0..10] == b"LEPCC     " {
        return Ok(BlobType::Xyz);
    }

    // Check RGB
    if data.len() >= 10 && &data[0..10] == b"ClusterRGB" {
        return Ok(BlobType::Rgb);
    }

    // Check Intensity
    if data.len() >= 10 && &data[0..10] == b"Intensity " {
        return Ok(BlobType::Intensity);
    }

    // Check FlagBytes
    if data.len() >= 10 && &data[0..10] == b"FlagBytes " {
        return Ok(BlobType::FlagBytes);
    }

    Err(LepccError::NotLepcc("Unknown blob type".to_string()))
}

/// Get the size of a compressed blob
///
/// # Arguments
///
/// * `data` - Compressed binary data
///
/// # Returns
///
/// Size of the blob in bytes
pub fn get_blob_size(data: &[u8]) -> Result<u32> {
    let blob_type = get_blob_type(data)?;

    match blob_type {
        BlobType::Xyz => LepccDecoder::get_blob_size(data),
        BlobType::Rgb => ClusterRgbDecoder::get_blob_size(data),
        BlobType::Intensity => IntensityDecoder::get_blob_size(data),
        BlobType::FlagBytes => super::flag_bytes::FlagBytesDecoder::get_blob_size(data),
    }
}

/// Get the number of points/values in compressed data
///
/// # Arguments
///
/// * `data` - Compressed binary data
///
/// # Returns
///
/// Number of points or values
pub fn get_num_points(data: &[u8]) -> Result<u32> {
    let blob_type = get_blob_type(data)?;

    match blob_type {
        BlobType::Xyz => LepccDecoder::get_num_points(data),
        BlobType::Rgb => ClusterRgbDecoder::get_num_points(data),
        BlobType::Intensity => IntensityDecoder::get_num_points(data),
        BlobType::FlagBytes => super::flag_bytes::FlagBytesDecoder::get_num_points(data),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_decompress_xyz_array() {
        let xyz_array = vec![
            0.0, 0.0, 0.0,  // Point 0
            1.0, 1.0, 1.0,  // Point 1
            2.0, 0.0, 1.0,  // Point 2
        ];

        let compressed = compress_xyz_array(&xyz_array, 0.01, 0.01, 0.01).unwrap();
        let decompressed = decompress_xyz_array(&compressed).unwrap();

        assert_eq!(decompressed.len(), xyz_array.len());
    }

    #[test]
    fn test_compress_decompress_rgb_array() {
        let rgb_array = vec![
            255, 0, 0,   // Red
            0, 255, 0,   // Green
            0, 0, 255,   // Blue
        ];

        let compressed = compress_rgb_array(&rgb_array).unwrap();
        let decompressed = decompress_rgb_array(&compressed).unwrap();

        assert_eq!(decompressed.len(), rgb_array.len());
    }

    #[test]
    fn test_get_blob_type_xyz() {
        let vertices = vec![Point3D::origin()];
        let compressed = compress_xyz(&vertices, 0.01, 0.01, 0.01).unwrap();

        let blob_type = get_blob_type(&compressed).unwrap();
        assert_eq!(blob_type, BlobType::Xyz);
    }

    #[test]
    fn test_get_blob_type_rgb() {
        let colors = vec![RGB::new(255, 0, 0)];
        let compressed = compress_rgb(&colors).unwrap();

        let blob_type = get_blob_type(&compressed).unwrap();
        assert_eq!(blob_type, BlobType::Rgb);
    }

    #[test]
    fn test_invalid_array_length_xyz() {
        let invalid_array = vec![0.0, 0.0]; // Not a multiple of 3

        let result = compress_xyz_array(&invalid_array, 0.01, 0.01, 0.01);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_array_length_rgb() {
        let invalid_array = vec![255u8, 0]; // Not a multiple of 3

        let result = compress_rgb_array(&invalid_array);
        assert!(result.is_err());
    }
}
