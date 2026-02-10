// Encode 3857 coordinates
use std::io::Read;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Write};

fn read_point_file(filename: &str) -> io::Result<Vec<[f64; 3]>> {
    let file = File::open(filename)?;
    let mut reader = BufReader::new(file);

    let mut num_bytes = [0u8; 4];
    reader.read_exact(&mut num_bytes)?;
    let num_points = u32::from_le_bytes(num_bytes) as usize;

    println!("Reading {} points from {}...", num_points, filename);

    let mut points = Vec::with_capacity(num_points);
    for _ in 0..num_points {
        let mut point = [0.0f64; 3];

        for i in 0..3 {
            let mut coord_bytes = [0u8; 8];
            reader.read_exact(&mut coord_bytes)?;
            point[i] = f64::from_le_bytes(coord_bytes);
        }

        points.push(point);
    }

    Ok(points)
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        println!("Usage: {} <input_bin_file>", args[0]);
        std::process::exit(1);
    }

    let input_file = &args[1];
    let output_file = "arcgis_3857_rust_encoded.pccxyz";

    let points = read_point_file(input_file)?;
    println!("Encoding {} points...", points.len());

    // Compute extent to verify
    let (mut x_min, mut x_max) = (points[0][0], points[0][0]);
    let (mut y_min, mut y_max) = (points[0][1], points[0][1]);
    let (mut z_min, mut z_max) = (points[0][2], points[0][2]);
    
    for p in &points {
        x_min = x_min.min(p[0]);
        x_max = x_max.max(p[0]);
        y_min = y_min.min(p[1]);
        y_max = y_max.max(p[1]);
        z_min = z_min.min(p[2]);
        z_max = z_max.max(p[2]);
    }
    
    println!("\nExtent from binary:");
    println!("  X: [{:.10}, {:.10}]", x_min, x_max);
    println!("  Y: [{:.10}, {:.10}]", y_min, y_max);
    println!("  Z: [{:.10}, {:.10}]", z_min, z_max);
    println!("  Size: X={:.2}, Y={:.2}, Z={:.2}", 
        x_max - x_min, y_max - y_min, z_max - z_min);

    let point3d_vec: Vec<lepcc::Point3D> = points
        .iter()
        .map(|p| lepcc::Point3D::new(p[0], p[1], p[2]))
        .collect();

    // Use max error from original file
    let max_err = 0.0027836876;
    let result = lepcc::api::compress_xyz(&point3d_vec, max_err, max_err, max_err);

    match result {
        Ok(compressed) => {
            println!(
                "\nCompressed to {} bytes (ratio: {:.2}x)",
                compressed.len(),
                (point3d_vec.len() * 24) as f64 / compressed.len() as f64
            );

            let file = File::create(output_file)?;
            let mut writer = BufWriter::new(file);
            writer.write_all(&compressed)?;
            writer.flush()?;

            println!("Encoded to {} ({} bytes)", output_file, compressed.len());

            // Print hex of first 64 bytes
            println!("\nFirst 64 bytes:")
                ;
            for i in (0..64usize).step_by(16) {
                let end = (i + 16).min(compressed.len());
                print!("{:04x}:", i);
                for j in i..end {
                    print!(" {:02x}", compressed[j]);
                }
                println!();
            }
        }
        Err(e) => {
            eprintln!("Encoding failed: {:?}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
