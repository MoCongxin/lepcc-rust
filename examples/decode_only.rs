use std::env;
use std::fs;
use lepcc::api::decompress_xyz;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <compressed_bin_file>", args[0]);
        std::process::exit(1);
    }

    let input_file = &args[1];
    println!("Reading compressed data from {}...", input_file);

    let compressed_data = match fs::read(input_file) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error: Cannot read file {}: {}", input_file, e);
            std::process::exit(1);
        }
    };

    println!("Compressed size: {} bytes", compressed_data.len());

    match decompress_xyz(&compressed_data) {
        Ok(points) => {
            println!("Decoded {} points", points.len());
            println!("First 3 points:");
            for (i, p) in points.iter().take(3).enumerate() {
                println!("  [{}] ({}, {}, {})", i, p.x, p.y, p.z);
            }
        }
        Err(e) => {
            eprintln!("Error: Decompression failed: {}", e);
            std::process::exit(1);
        }
    }
}
