// Debug version of lepcc_xyz.rs with detailed logging

use crate::bit_stuffer2::BitStuffer2;
use crate::common::compute_checksum_fletcher32;
use crate::error::{LepccError, Result};
use crate::types::{Byte, Extent3D, Point3D};
use std::io::{Cursor, Write};

// ... (Cell3D and comparison function remain the same)

impl LepccEncoder {
    pub fn encode(&self) -> Result<Vec<Byte>> {
        let mut buffer = Cursor::new(Vec::new());

        // Write TopHeader
        buffer.write_all(b"LEPCC     ")?;
        buffer.write_all(&1u16.to_le_bytes())?;
        buffer.write_all(&0u32.to_le_bytes())?; // checksum (filled later)

        // Write Header1
        let blob_size_pos = buffer.position() as usize;
        buffer.write_all(&0i64.to_le_bytes())?; // blob_size placeholder

        // Write extent
        self.write_point_3d(&mut buffer, &self.extent_3d.lower)?;
        self.write_point_3d(&mut buffer, &self.extent_3d.upper)?;

        // Write reserved (8 bytes)
        buffer.write_all(&0u64.to_le_bytes())?;

        // Write max error
        self.write_point_3d(&mut buffer, &self.max_error)?;

        // Write num points
        buffer.write_all(&(self.z_vec.len() as u32).to_le_bytes())?;

        // Write reserved (4 bytes)
        buffer.write_all(&0u32.to_le_bytes())?;

        eprintln!("\n=== ENCODING SECTIONS ===");

        // Encode data segments WITH DEBUG
        let y_encoded = self.encode_cut_in_segments_debug(&self.y_delta_vec, "y_delta")?;
        buffer.write_all(&y_encoded)?;

        let row_encoded =
            self.encode_cut_in_segments_debug(&self.num_points_per_row_vec, "num_points_per_row")?;
        buffer.write_all(&row_encoded)?;

        let x_encoded = self.encode_cut_in_segments_debug(&self.x_delta_vec, "x_delta")?;
        buffer.write_all(&x_encoded)?;

        let z_encoded = self.encode_cut_in_segments_debug(&self.z_vec, "z")?;
        buffer.write_all(&z_encoded)?;

        // Update blob_size
        let mut result = buffer.into_inner();
        let blob_size = result.len() as i64;
        result[blob_size_pos..blob_size_pos + 8].copy_from_slice(&blob_size.to_le_bytes());

        // Compute and write checksum
        let checksum = compute_checksum_fletcher32(&result[16..blob_size as usize]);
        result[12..16].copy_from_slice(&checksum.to_le_bytes());

        eprintln!("\n=== FINAL STATS ===");
        eprintln!(
            "Blob size: {}, Expected: {}",
            blob_size, self.num_bytes_needed
        );
        eprintln!("Checksum: {:08x}", checksum);

        if blob_size != self.num_bytes_needed {
            eprintln!("WARNING: Size mismatch!");
        }

        Ok(result)
    }

    fn encode_cut_in_segments_debug(&self, data_vec: &[u32], name: &str) -> Result<Vec<Byte>> {
        eprintln!("\n--- Encoding {} ({} elements) ---", name, data_vec.len());

        let num_sections =
            (data_vec.len() + (self.section_size as usize - 1)) / self.section_size as usize;

        eprintln!("Number of sections: {}", num_sections);

        let mut section_min_vec = Vec::new();
        let mut total_max = 0u32;

        for i in 0..num_sections {
            let start = i * self.section_size as usize;
            let end = std::cmp::min(start + self.section_size as usize, data_vec.len());

            let mut min_elem = data_vec[start];
            let mut max_elem = min_elem;

            for j in start..end {
                min_elem = min_elem.min(data_vec[j]);
                max_elem = max_elem.max(data_vec[j]);
            }

            section_min_vec.push(min_elem);
            total_max = total_max.max(max_elem);

            eprintln!(
                "  Section {}: len={}, min={}, max={}",
                i,
                end - start,
                min_elem,
                max_elem
            );
        }

        let max_of_section_mins = *section_min_vec.iter().max().unwrap_or(&0);
        eprintln!("Max of section mins: {}", max_of_section_mins);
        eprintln!("Total max: {}", total_max);
        eprintln!("Section mins: {:?}", section_min_vec);

        // Write section mins
        eprintln!(
            "\nEncoding section mins ({} elements):",
            section_min_vec.len()
        );
        let encoded_mins = BitStuffer2::encode_simple(&section_min_vec)?;
        eprintln!("  Encoded size: {} bytes", encoded_mins.len());
        eprintln!(
            "  First 8 bytes: {:?}",
            &encoded_mins[..encoded_mins.len().min(8)]
        );

        // Write sections
        for i in 0..num_sections {
            let start = i * self.section_size as usize;
            let end = std::cmp::min(start + self.section_size as usize, data_vec.len());
            let min_elem = section_min_vec[i];

            let mut zero_based_data: Vec<u32> = Vec::new();
            for j in start..end {
                zero_based_data.push(data_vec[j] - min_elem);
            }

            eprintln!(
                "\nEncoding section {} ({} elements, min={}):",
                i,
                zero_based_data.len(),
                min_elem
            );
            let encoded = BitStuffer2::encode_simple(&zero_based_data)?;
            eprintln!("  Encoded size: {} bytes", encoded.len());
            eprintln!("  First 8 bytes: {:?}", &encoded[..encoded.len().min(8)]);
        }

        // Note: In actual code, buffer.write_all calls here
        // For debug, we'll just return the concatenated data
        let mut result = Vec::new();
        result.extend_from_slice(&encoded_mins);

        for i in 0..num_sections {
            let start = i * self.section_size as usize;
            let end = std::cmp::min(start + self.section_size as usize, data_vec.len());
            let min_elem = section_min_vec[i];

            let mut zero_based_data: Vec<u32> = Vec::new();
            for j in start..end {
                zero_based_data.push(data_vec[j] - min_elem);
            }

            let encoded = BitStuffer2::encode_simple(&zero_based_data)?;
            result.extend_from_slice(&encoded);
        }

        eprintln!("Total encoded size for {}: {} bytes", name, result.len());

        Ok(result)
    }

    // ... rest of the implementation remains the same as non-debug version
}
