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

//! Error types for LEPCC operations

use std::fmt;
use crate::types::ErrCode;

/// Error type for LEPCC operations
#[derive(Debug, Clone, PartialEq)]
pub enum LepccError {
    /// Generic failure
    Failed(String),
    /// Wrong parameter
    WrongParam(String),
    /// Wrong version
    WrongVersion {
        found: u16,
        expected: u16,
    },
    /// Wrong checksum
    WrongChecksum {
        expected: u32,
        found: u32,
    },
    /// Not valid LEPCC data
    NotLepcc(String),
    NotClusterRgb(String),
    NotIntensity(String),
    NotFlagBytes(String),
    /// Buffer too small
    BufferTooSmall {
        needed: usize,
        provided: usize,
    },
    /// Output array too small
    OutArrayTooSmall {
        needed: usize,
        provided: usize,
    },
    /// Quantize virtual raster too big
    QuantizeVirtualRasterTooBig,
    /// Quantize index out of range
    QuantizeIndexOutOfRange {
        index: i64,
        limit: usize,
    },
    /// I/O error
    IoError(String),
}

impl fmt::Display for LepccError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LepccError::Failed(msg) => write!(f, "Failed: {}", msg),
            LepccError::WrongParam(msg) => write!(f, "Wrong Parameter: {}", msg),
            LepccError::WrongVersion { found, expected } => {
                write!(f, "Wrong Version: found {}, expected {}", found, expected)
            }
            LepccError::WrongChecksum { expected, found } => {
                write!(f, "Wrong Checksum: expected 0x{:08x}, found 0x{:08x}", expected, found)
            }
            LepccError::NotLepcc(msg) => write!(f, "Not Lepcc: {}", msg),
            LepccError::NotClusterRgb(msg) => write!(f, "Not ClusterRgb: {}", msg),
            LepccError::NotIntensity(msg) => write!(f, "Not Intensity: {}", msg),
            LepccError::NotFlagBytes(msg) => write!(f, "Not FlagBytes: {}", msg),
            LepccError::BufferTooSmall { needed, provided } => {
                write!(f, "Buffer Too Small: needed {} bytes, provided {} bytes", needed, provided)
            }
            LepccError::OutArrayTooSmall { needed, provided } => {
                write!(f, "Output Array Too Small: needed {} elements, provided {}", needed, provided)
            }
            LepccError::QuantizeVirtualRasterTooBig => {
                write!(f, "Quantize Virtual Raster Too Big")
            }
            LepccError::QuantizeIndexOutOfRange { index, limit } => {
                write!(f, "Quantize Index Out Of Range: index {}, limit {}", index, limit)
            }
            LepccError::IoError(msg) => write!(f, "I/O Error: {}", msg),
        }
    }
}

impl std::error::Error for LepccError {}

impl From<ErrCode> for LepccError {
    fn from(code: ErrCode) -> Self {
        match code {
            ErrCode::Ok => unreachable!("Ok should not be converted to error"),
            ErrCode::Failed => LepccError::Failed("Operation failed".to_string()),
            ErrCode::WrongParam => LepccError::WrongParam("Invalid parameter".to_string()),
            ErrCode::WrongVersion => LepccError::WrongVersion { found: 0, expected: 1 },
            ErrCode::WrongChecksum => LepccError::WrongChecksum { expected: 0, found: 0 },
            ErrCode::NotLepcc => LepccError::NotLepcc("Invalid magic bytes".to_string()),
            ErrCode::NotClusterRgb => LepccError::NotClusterRgb("Not ClusterRGB data".to_string()),
            ErrCode::NotIntensity => LepccError::NotIntensity("Not Intensity data".to_string()),
            ErrCode::NotFlagBytes => LepccError::NotFlagBytes("Not FlagBytes data".to_string()),
            ErrCode::BufferTooSmall => LepccError::BufferTooSmall { needed: 0, provided: 0 },
            ErrCode::OutArrayTooSmall => LepccError::OutArrayTooSmall { needed: 0, provided: 0 },
            ErrCode::QuantizeVirtualRasterTooBig => LepccError::QuantizeVirtualRasterTooBig,
            ErrCode::QuantizeIndexOutOfRange => LepccError::QuantizeIndexOutOfRange {
                index: 0,
                limit: 0,
            },
        }
    }
}

impl From<std::io::Error> for LepccError {
    fn from(err: std::io::Error) -> Self {
        LepccError::IoError(err.to_string())
    }
}

impl std::convert::TryFrom<LepccError> for ErrCode {
    type Error = ();

    fn try_from(error: LepccError) -> std::result::Result<Self, Self::Error> {
        match error {
            LepccError::Failed(_) => Ok(ErrCode::Failed),
            LepccError::WrongParam(_) => Ok(ErrCode::WrongParam),
            LepccError::WrongVersion { .. } => Ok(ErrCode::WrongVersion),
            LepccError::WrongChecksum { .. } => Ok(ErrCode::WrongChecksum),
            LepccError::NotLepcc(_) => Ok(ErrCode::NotLepcc),
            LepccError::NotClusterRgb(_) => Ok(ErrCode::NotClusterRgb),
            LepccError::NotIntensity(_) => Ok(ErrCode::NotIntensity),
            LepccError::NotFlagBytes(_) => Ok(ErrCode::NotFlagBytes),
            LepccError::BufferTooSmall { .. } => Ok(ErrCode::BufferTooSmall),
            LepccError::OutArrayTooSmall { .. } => Ok(ErrCode::OutArrayTooSmall),
            LepccError::QuantizeVirtualRasterTooBig => Ok(ErrCode::QuantizeVirtualRasterTooBig),
            LepccError::QuantizeIndexOutOfRange { .. } => Ok(ErrCode::QuantizeIndexOutOfRange),
            LepccError::IoError(_) => Err(()),
        }
    }
}

/// Result type alias for LEPCC operations
pub type Result<T> = std::result::Result<T, LepccError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = LepccError::WrongParam("test message".to_string());
        assert_eq!(format!("{}", err), "Wrong Parameter: test message");
    }

    #[test]
    fn test_error_conversion_from_errcode() {
        let err = LepccError::from(ErrCode::WrongParam);
        assert!(matches!(err, LepccError::WrongParam(_)));
    }
}
