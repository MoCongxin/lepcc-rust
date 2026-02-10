use std::io::Read;
// Simple program to just compress points (no decode)
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
        println!("Example: {} test_input.bin", args[0]);
        std::process::exit(1);
    }

    let input_file = &args[1];

    let points = read_point_file(input_file)?;
    println!("Encoding {} points...", points.len());

    let point3d_vec: Vec<lepcc::Point3D> = points
        .iter()
        .map(|p| lepcc::Point3D::new(p[0], p[1], p[2]))
        .collect();

    let result = lepcc::api::compress_xyz(&point3d_vec, 0.0000000250, 0.0000000250, 0.0027836876);

    match result {
        Ok(compressed) => {
            println!(
                "Compressed to {} bytes (ratio: {:.2}x)",
                compressed.len(),
                (point3d_vec.len() * 24) as f64 / compressed.len() as f64
            );

            let output_file = "arcgis_reencoded_rust.pccxyz";
            let file = File::create(output_file)?;
            let mut writer = BufWriter::new(file);
            writer.write_all(&compressed)?;

            println!("Encoded to {} ({} bytes)", output_file, compressed.len());

            // Print hex of first 64 bytes for comparison
            println!("\nFirst 64 bytes:");
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
