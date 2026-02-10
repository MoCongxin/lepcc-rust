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

//! Type definitions for LEPCC library

use std::fmt;

/// Byte type alias
pub type Byte = u8;

/// Unsigned 16-bit integer
pub type U16 = u16;

/// Unsigned 32-bit integer
pub type U32 = u32;

/// Unsigned 64-bit integer
pub type U64 = u64;

/// Signed 64-bit integer
pub type I64 = i64;

/// RGB color representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct RGB {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RGB {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        RGB { r, g, b }
    }

    pub fn black() -> Self {
        RGB::new(0, 0, 0)
    }

    pub fn white() -> Self {
        RGB::new(255, 255, 255)
    }
}

impl Default for RGB {
    fn default() -> Self {
        RGB::new(0, 0, 0)
    }
}

impl fmt::Display for RGB {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RGB({}, {}, {})", self.r, self.g, self.b)
    }
}

/// 3D point representation
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Point3D {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Point3D {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Point3D { x, y, z }
    }

    pub fn origin() -> Self {
        Point3D::new(0.0, 0.0, 0.0)
    }

    pub fn from_xyz_array(arr: &[f64]) -> Self {
        Point3D {
            x: arr[0],
            y: arr[1],
            z: arr[2],
        }
    }

    pub fn to_xyz_array(&self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }
}

impl Default for Point3D {
    fn default() -> Self {
        Point3D::origin()
    }
}

impl std::ops::Sub for Point3D {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Point3D {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl fmt::Display for Point3D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

/// 3D Extent (bounding box)
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Extent3D {
    pub lower: Point3D,
    pub upper: Point3D,
}

impl Extent3D {
    pub fn new(lower: Point3D, upper: Point3D) -> Self {
        Extent3D { lower, upper }
    }

    pub fn empty() -> Self {
        let origin = Point3D::origin();
        Extent3D {
            lower: origin,
            upper: origin,
        }
    }

    pub fn contains(&self, point: &Point3D) -> bool {
        point.x >= self.lower.x
            && point.x <= self.upper.x
            && point.y >= self.lower.y
            && point.y <= self.upper.y
            && point.z >= self.lower.z
            && point.z <= self.upper.z
    }

    pub fn size(&self) -> Point3D {
        Point3D {
            x: self.upper.x - self.lower.x,
            y: self.upper.y - self.lower.y,
            z: self.upper.z - self.lower.z,
        }
    }
}

impl Default for Extent3D {
    fn default() -> Self {
        Extent3D::empty()
    }
}

/// Error codes for LEPCC operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrCode {
    Ok = 0,
    Failed = 1,
    WrongParam = 2,
    WrongVersion = 3,
    WrongChecksum = 4,
    NotLepcc = 5,
    NotClusterRgb = 6,
    NotIntensity = 7,
    NotFlagBytes = 8,
    BufferTooSmall = 9,
    OutArrayTooSmall = 10,
    QuantizeVirtualRasterTooBig = 11,
    QuantizeIndexOutOfRange = 12,
}

impl fmt::Display for ErrCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            ErrCode::Ok => "Ok",
            ErrCode::Failed => "Failed",
            ErrCode::WrongParam => "Wrong Parameter",
            ErrCode::WrongVersion => "Wrong Version",
            ErrCode::WrongChecksum => "Wrong Checksum",
            ErrCode::NotLepcc => "Not LEPCC data",
            ErrCode::NotClusterRgb => "Not ClusterRGB data",
            ErrCode::NotIntensity => "Not Intensity data",
            ErrCode::NotFlagBytes => "Not FlagBytes data",
            ErrCode::BufferTooSmall => "Buffer Too Small",
            ErrCode::OutArrayTooSmall => "Output Array Too Small",
            ErrCode::QuantizeVirtualRasterTooBig => "Quantize Virtual Raster Too Big",
            ErrCode::QuantizeIndexOutOfRange => "Quantize Index Out Of Range",
        };
        write!(f, "{}", msg)
    }
}

/// Blob type identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlobType {
    Xyz = 0,
    Rgb = 1,
    Intensity = 2,
    FlagBytes = 3,
}

impl fmt::Display for BlobType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            BlobType::Xyz => "XYZ",
            BlobType::Rgb => "RGB",
            BlobType::Intensity => "Intensity",
            BlobType::FlagBytes => "FlagBytes",
        };
        write!(f, "{}", name)
    }
}

impl TryFrom<u32> for BlobType {
    type Error = ErrCode;

    fn try_from(value: u32) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(BlobType::Xyz),
            1 => Ok(BlobType::Rgb),
            2 => Ok(BlobType::Intensity),
            3 => Ok(BlobType::FlagBytes),
            _ => Err(ErrCode::WrongParam),
        }
    }
}

impl From<BlobType> for u32 {
    fn from(blob_type: BlobType) -> Self {
        blob_type as u32
    }
}

/// Wrapper type for flat XYZ arrays
///
/// Converts between &[f64] and &[Point3D] representations
pub struct FlatXyzSlice<'a> {
    pub data: &'a [f64],
}

impl<'a> FlatXyzSlice<'a> {
    pub fn from_slice(data: &'a [f64]) -> Self {
        Self { data }
    }

    pub fn len(&self) -> usize {
        self.data.len() / 3
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, index: usize) -> Option<Point3D> {
        if index * 3 + 2 < self.data.len() {
            Some(Point3D {
                x: self.data[index * 3],
                y: self.data[index * 3 + 1],
                z: self.data[index * 3 + 2],
            })
        } else {
            None
        }
    }

    pub fn iter(&'a self) -> FlatXyzIter<'a> {
        FlatXyzIter {
            data: self.data,
            index: 0,
        }
    }
}

pub struct FlatXyzIter<'a> {
    data: &'a [f64],
    index: usize,
}

impl<'a> Iterator for FlatXyzIter<'a> {
    type Item = Point3D;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index * 3 + 2 < self.data.len() {
            let result = Point3D {
                x: self.data[self.index * 3],
                y: self.data[self.index * 3 + 1],
                z: self.data[self.index * 3 + 2],
            };
            self.index += 1;
            Some(result)
        } else {
            None
        }
    }
}

/// Wrapper type for flat RGB arrays
pub struct FlatRgbSlice<'a> {
    pub data: &'a [u8],
}

impl<'a> FlatRgbSlice<'a> {
    pub fn from_slice(data: &'a [u8]) -> Self {
        Self { data }
    }

    pub fn len(&self) -> usize {
        self.data.len() / 3
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, index: usize) -> Option<RGB> {
        if index * 3 + 2 < self.data.len() {
            Some(RGB {
                r: self.data[index * 3],
                g: self.data[index * 3 + 1],
                b: self.data[index * 3 + 2],
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_eq() {
        let rgb1 = RGB { r: 255, g: 0, b: 0 };
        let rgb2 = RGB { r: 255, g: 0, b: 0 };
        let rgb3 = RGB { r: 0, g: 255, b: 0 };
        assert_eq!(rgb1, rgb2);
        assert_ne!(rgb1, rgb3);
    }

    #[test]
    fn test_point3d_sub() {
        let p1 = Point3D::new(5.0, 4.0, 3.0);
        let p2 = Point3D::new(1.0, 2.0, 1.0);
        let diff = p1 - p2;
        assert_eq!(diff.x, 4.0);
        assert_eq!(diff.y, 2.0);
        assert_eq!(diff.z, 2.0);
    }

    #[test]
    fn test_extent_contains() {
        let extent = Extent3D::new(
            Point3D::new(0.0, 0.0, 0.0),
            Point3D::new(10.0, 10.0, 10.0),
        );
        assert!(extent.contains(&Point3D::new(5.0, 5.0, 5.0)));
        assert!(!extent.contains(&Point3D::new(-1.0, 5.0, 5.0)));
        assert!(!extent.contains(&Point3D::new(11.0, 5.0, 5.0)));
    }

    #[test]
    fn test_flat_xyz_iter() {
        let data = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
        let slice = FlatXyzSlice::from_slice(&data);
        assert_eq!(slice.len(), 2);

        let points: Vec<Point3D> = slice.iter().collect();
        assert_eq!(points.len(), 2);
        assert_eq!(points[0], Point3D::new(0.0, 1.0, 2.0));
        assert_eq!(points[1], Point3D::new(3.0, 4.0, 5.0));
    }

    #[test]
    fn test_blob_type_conversion() {
        assert_eq!(BlobType::try_from(0).unwrap(), BlobType::Xyz);
        assert_eq!(BlobType::try_from(1).unwrap(), BlobType::Rgb);
        assert_eq!(BlobType::try_from(2).unwrap(), BlobType::Intensity);
        assert_eq!(BlobType::try_from(3).unwrap(), BlobType::FlagBytes);
        assert!(BlobType::try_from(4).is_err());

        assert_eq!(u32::from(BlobType::Xyz), 0);
        assert_eq!(u32::from(BlobType::Rgb), 1);
        assert_eq!(u32::from(BlobType::Intensity), 2);
        assert_eq!(u32::from(BlobType::FlagBytes), 3);
    }
}
