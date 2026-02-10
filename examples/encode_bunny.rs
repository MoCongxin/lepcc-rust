// Encode bunny.obj to LEPCC format and verify

use lepcc::prelude::*;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let obj_path = r"C:\Users\admin\Desktop\i3s文档\bunny.obj";
    let output_path = r"C:\Users\admin\Desktop\i3s文档\bunny_rust.lepcc";
    let decoded_txt_path = r"C:\Users\admin\Desktop\i3s文档\bunny_rust_decoded.txt";

    println!("Reading OBJ file: {}", obj_path);

    // Read OBJ file
    let file = File::open(obj_path)?;
    let reader = BufReader::new(file);
    let mut points = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse vertex lines (v x y z)
        if line.starts_with("v ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let x: f64 = parts[1]
                    .parse()
                    .map_err(|e| format!("Parse error: {}", e))?;
                let y: f64 = parts[2]
                    .parse()
                    .map_err(|e| format!("Parse error: {}", e))?;
                let z: f64 = parts[3]
                    .parse()
                    .map_err(|e| format!("Parse error: {}", e))?;
                points.push(Point3D::new(x, y, z));
            }
        }
    }

    println!("Loaded {} points from OBJ file", points.len());

    // Print first 5 points
    println!("\nFirst 5 points:");
    for (i, p) in points.iter().take(5).enumerate() {
        println!("  [{}] ({:.10}, {:.10}, {:.10})", i, p.x, p.y, p.z);
    }

    // Compute extent
    let (mut x_min, mut x_max) = (points[0].x, points[0].x);
    let (mut y_min, mut y_max) = (points[0].y, points[0].y);
    let (mut z_min, mut z_max) = (points[0].z, points[0].z);

    for p in &points {
        x_min = x_min.min(p.x);
        x_max = x_max.max(p.x);
        y_min = y_min.min(p.y);
        y_max = y_max.max(p.y);
        z_min = z_min.min(p.z);
        z_max = z_max.max(p.z);
    }

    println!("\nExtent:");
    println!("  X: [{:.10}, {:.10}]", x_min, x_max);
    println!("  Y: [{:.10}, {:.10}]", y_min, y_max);
    println!("  Z: [{:.10}, {:.10}]", z_min, z_max);

    // Compression parameters
    // Use small error values similar to ArcGIS
    let max_x_err = 0.00001; // Small error for X
    let max_y_err = 0.00001; // Small error for Y
    let max_z_err = 0.00001; // Small error for Z

    println!("\nCompression parameters:");
    println!("  max_x_err = {:.10}", max_x_err);
    println!("  max_y_err = {:.10}", max_y_err);
    println!("  max_z_err = {:.10}", max_z_err);

    // Compress
    println!("\nCompressing...");
    let compressed = compress_xyz(&points, max_x_err, max_y_err, max_z_err)?;

    println!(
        "Compressed size: {} bytes ({:.4} bytes/point)",
        compressed.len(),
        compressed.len() as f64 / points.len() as f64
    );

    // Write compressed file
    let mut output_file = File::create(output_path)?;
    output_file.write_all(&compressed)?;
    println!("\nWrote compressed data to: {}", output_path);

    // Print file header info
    println!("\n=== File Header ===");
    println!("File key: {}", String::from_utf8_lossy(&compressed[0..10]));
    println!(
        "Version: {}",
        u16::from_le_bytes([compressed[10], compressed[11]])
    );
    println!(
        "Checksum: 0x{:08X}",
        u32::from_le_bytes([
            compressed[12],
            compressed[13],
            compressed[14],
            compressed[15]
        ])
    );
    println!(
        "Blob size: {}",
        i64::from_le_bytes([
            compressed[16],
            compressed[17],
            compressed[18],
            compressed[19],
            compressed[20],
            compressed[21],
            compressed[22],
            compressed[23]
        ])
    );

    // Decompress for verification
    println!("\n=== Verification ===");
    let decompressed = decompress_xyz(&compressed)?;

    println!("Decompressed {} points", decompressed.len());

    // Print first 5 decompressed points
    println!("\nFirst 5 decompressed points:");
    for (i, p) in decompressed.iter().take(5).enumerate() {
        println!("  [{}] ({:.10}, {:.10}, {:.10})", i, p.x, p.y, p.z);
    }

    // Calculate errors
    let mut max_err_x = 0.0f64;
    let mut max_err_y = 0.0f64;
    let mut max_err_z = 0.0f64;
    let mut total_err_x = 0.0f64;
    let mut total_err_y = 0.0f64;
    let mut total_err_z = 0.0f64;

    for (orig, dec) in points.iter().zip(decompressed.iter()) {
        let err_x = (orig.x - dec.x).abs();
        let err_y = (orig.y - dec.y).abs();
        let err_z = (orig.z - dec.z).abs();

        max_err_x = max_err_x.max(err_x);
        max_err_y = max_err_y.max(err_y);
        max_err_z = max_err_z.max(err_z);

        total_err_x += err_x;
        total_err_y += err_y;
        total_err_z += err_z;
    }

    let n = points.len() as f64;
    println!("\nError analysis:");
    println!(
        "  Max errors: X={:.10}, Y={:.10}, Z={:.10}",
        max_err_x, max_err_y, max_err_z
    );
    println!(
        "  Avg errors: X={:.10}, Y={:.10}, Z={:.10}",
        total_err_x / n,
        total_err_y / n,
        total_err_z / n
    );
    println!(
        "  Error limits: X={:.10}, Y={:.10}, Z={:.10}",
        max_x_err * 2.0,
        max_y_err * 2.0,
        max_z_err * 2.0
    );

    // Check if within limits
    if max_err_x <= max_x_err * 2.0 + 1e-10
        && max_err_y <= max_y_err * 2.0 + 1e-10
        && max_err_z <= max_z_err * 2.0 + 1e-10
    {
        println!("\n✓ All errors within limits!");
    } else {
        println!("\n✗ Some errors exceed limits!");
    }

    // Write decoded points to text file
    let mut txt_file = File::create(decoded_txt_path)?;
    writeln!(txt_file, "# Decoded from Rust LEPCC encoding")?;
    writeln!(txt_file, "# Points: {}", decompressed.len())?;
    writeln!(
        txt_file,
        "# Max errors: X={:.10}, Y={:.10}, Z={:.10}",
        max_err_x, max_err_y, max_err_z
    )?;
    writeln!(txt_file)?;

    for p in &decompressed {
        writeln!(txt_file, "{:.10} {:.10} {:.10}", p.x, p.y, p.z)?;
    }

    println!("\nWrote decoded points to: {}", decoded_txt_path);

    Ok(())
}
