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

//! # LEPCC - Limited Error Point Cloud Compression
//!
//! A Rust implementation of the LEPCC compression algorithm originally
//! developed by Esri. This library provides lossless compression for
//! 3D point cloud data including coordinates, colors, intensity, and
//! flag bytes.
//!
//! ## Features
//!
//! - **XYZ Compression**: Lossy compression of 3D coordinates with configurable error tolerance
//! - **RGB Compression**: Color clustering and quantization with up to 3x compression
//! - **Intensity Compression**: Lossless compression of intensity values
//! - **Flag Bytes Compression**: Lossless compression of flag/class data
//! - **I3S Format Support**: Compatible with I3S point cloud format (.pccxyz and .pccrgb)
//!
//! ## Example
//!
//! ```ignore
//! use lepcc::prelude::*;
//!
//! // Compress XYZ coordinates with 1cm accuracy
//! let points = vec![
//!     [0.0, 0.0, 0.0],
//!     [1.0, 1.0, 1.0],
//!     // ... more points
//! ];
//! let compressed = compress_xyz(&points, 0.01, 0.01, 0.01)?;
//!
//! // Decompress
//! let decompressed = decompress_xyz(&compressed)?;
//! ```

pub mod error;
pub mod types;
pub mod bit_mask;
pub mod bit_stuffer2;
pub mod common;
pub mod huffman;
pub mod lepcc_xyz;
pub mod cluster_rgb;
pub mod intensity;
pub mod flag_bytes;

pub mod api;

pub use error::{LepccError, Result};
pub use types::*;
pub use api::*;

/// Prelude module for common imports
pub mod prelude {
    pub use crate::error::{LepccError, Result};
    pub use crate::types::*;
    pub use crate::api::*;
}

// Version constants
pub const VERSION_LEPCC: u32 = 1;
pub const VERSION_LEPCC_XYZ: u32 = 1;
pub const VERSION_LEPCC_RGB: u32 = 1;
pub const VERSION_LEPCC_INTENSITY: u32 = 1;
pub const VERSION_LEPCC_FLAGBYTES: u32 = 1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_roundtrip_simple_xyz() {
        // Simple test for XYZ compression
        let points = vec![
            Point3D { x: 0.0, y: 0.0, z: 0.0 },
            Point3D { x: 1.0, y: 0.0, z: 0.0 },
            Point3D { x: 0.0, y: 1.0, z: 0.0 },
            Point3D { x: 1.0, y: 1.0, z: 1.0 },
        ];

        let max_err = 0.01;
        let result = compress_xyz(&points, max_err, max_err, max_err);
        assert!(result.is_ok());

        let compressed = result.unwrap();
        let decompressed = decompress_xyz(&compressed);
        assert!(decompressed.is_ok());

        let recovered = decompressed.unwrap();
        assert_eq!(recovered.len(), points.len());

        // Check points are within error tolerance
        for (orig, rec) in points.iter().zip(recovered.iter()) {
            let dx = (orig.x - rec.x).abs();
            let dy = (orig.y - rec.y).abs();
            let dz = (orig.z - rec.z).abs();
            assert!(dx <= max_err * 2.0, "X error too large: {}", dx);
            assert!(dy <= max_err * 2.0, "Y error too large: {}", dy);
            assert!(dz <= max_err * 2.0, "Z error too large: {}", dz);
        }
    }

    #[test]
    fn test_compression_roundtrip_simple_rgb() {
        let colors = vec![
            RGB { r: 255, g: 0, b: 0 },
            RGB { r: 0, g: 255, b: 0 },
            RGB { r: 0, g: 0, b: 255 },
        ];

        let result = compress_rgb(&colors);
        assert!(result.is_ok());

        let compressed = result.unwrap();
        let decompressed = decompress_rgb(&compressed);
        assert!(decompressed.is_ok());

        let recovered = decompressed.unwrap();
        assert_eq!(recovered.len(), colors.len());

        // RGB compression should be lossless for small datasets
        for (orig, rec) in colors.iter().zip(recovered.iter()) {
            assert_eq!(orig, rec);
        }
    }
}
