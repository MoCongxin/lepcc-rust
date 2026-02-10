use std::fs::File;
use std::io::{BufRead, BufReader, Write};

fn compute_extent(points: &[f64], stride: usize) -> (f64, f64) {
    let mut min = points[0];
    let mut max = points[0];
    for i in (0..points.len()).step_by(stride) {
        min = min.min(points[i]);
        max = max.max(points[i]);
    }
    (min, max)
}

fn read_obj_file(
    filename: &str,
) -> Result<(Vec<f64>, Vec<f64>, Vec<f64>), Box<dyn std::error::Error>> {
    let file = File::open(filename)?;
    let reader = BufReader::new(file);
    let mut x_vals = Vec::new();
    let mut y_vals = Vec::new();
    let mut z_vals = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();

        if line.starts_with("v ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let x: f64 = parts[1].parse()?;
                let y: f64 = parts[2].parse()?;
                let z: f64 = parts[3].parse()?;
                x_vals.push(x);
                y_vals.push(y);
                z_vals.push(z);
            }
        }
    }

    Ok((x_vals, y_vals, z_vals))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input_file = r"C:\Users\admin\Desktop\i3s文档\bunny_offset_scaled.obj";

    println!("Reading OBJ file: {}", input_file);
    let (x_vals, y_vals, z_vals) = read_obj_file(input_file)?;
    println!("Loaded {} points", x_vals.len());

    // Compute extent exactly like C++
    let (x_min, x_max) = compute_extent(&x_vals, 1);
    let (y_min, y_max) = compute_extent(&y_vals, 1);
    let (z_min, z_max) = compute_extent(&z_vals, 1);

    println!("\nBounding box:");
    println!(
        "  X: [{:.10}, {:.10}] (span: {:.10})",
        x_min,
        x_max,
        x_max - x_min
    );
    println!(
        "  Y: [{:.10}, {:.10}] (span: {:.10})",
        y_min,
        y_max,
        y_max - y_min
    );
    println!(
        "  Z: [{:.10}, {:.10}] (span: {:.10})",
        z_min,
        z_max,
        z_max - z_min
    );

    // Compute max errors exactly like C++
    let extent_x = x_max - x_min;
    let extent_y = y_max - y_min;
    let extent_z = z_max - z_min;

    let mut max_err_x = extent_x * 0.00001;
    let mut max_err_y = extent_y * 0.00001;
    let mut max_err_z = extent_z * 0.001;

    // Apply minimum thresholds (C++ code does this)
    max_err_x = max_err_x.max(1e-6);
    max_err_y = max_err_y.max(1e-6);
    max_err_z = max_err_z.max(1e-3);

    println!("\nCompression tolerances:");
    println!("  MaxErrorX: {:.16}", max_err_x);
    println!("  MaxErrorY: {:.16}", max_err_y);
    println!("  MaxErrorZ: {:.16}", max_err_z);

    // Create Point3D vector
    let points: Vec<lepcc::Point3D> = x_vals
        .iter()
        .zip(y_vals.iter())
        .zip(z_vals.iter())
        .map(|((&x, &y), &z)| lepcc::Point3D::new(x, y, z))
        .collect();

    // Encode
    println!("\nEncoding...");
    let compressed = lepcc::api::compress_xyz(&points, max_err_x, max_err_y, max_err_z)?;

    println!("Computed compressed size: {} bytes", compressed.len());
    println!(
        "Compression ratio: {:.2}%",
        (points.len() as f64 * 24.0) / compressed.len() as f64 * 100.0
    );

    // Write output
    let output_file = r"C:\Users\admin\Desktop\i3s文档\bunny_offset_scaled_rust.pccxyz";
    let file = File::create(output_file)?;
    let mut writer = std::io::BufWriter::new(file);
    writer.write_all(&compressed)?;
    drop(writer);

    println!(
        "\nEncoded {} points into {} bytes",
        points.len(),
        compressed.len()
    );
    println!("Wrote to {}", output_file);

    // Print header for verification
    println!("\n=== Header Info ===");
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

    Ok(())
}
