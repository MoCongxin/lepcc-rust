use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};

fn read_obj_file(filename: &str) -> Result<Vec<lepcc::Point3D>, Box<dyn std::error::Error>> {
    let file = File::open(filename)?;
    let reader = BufReader::new(file);
    let mut points = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();

        if line.starts_with("v ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let x: f64 = parts[1].parse()?;
                let y: f64 = parts[2].parse()?;
                let z: f64 = parts[3].parse()?;
                points.push(lepcc::Point3D::new(x, y, z));
            }
        }
    }

    Ok(points)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 4 {
        println!(
            "Usage: {} <input.obj> <max_x_err> <max_y_err> <max_z_err>",
            args[0]
        );
        println!("Example: {} input.obj 0.0017 0.0015 0.0273", args[0]);
        std::process::exit(1);
    }

    let input_file = &args[1];
    let max_x_err: f64 = args[2].parse()?;
    let max_y_err: f64 = args[3].parse()?;
    let max_z_err: f64 = args[4].parse()?;

    println!("Reading OBJ file: {}", input_file);
    let points = read_obj_file(input_file)?;
    println!("Loaded {} points", points.len());

    println!(
        "Encoding with max errors: X={}, Y={}, Z={}",
        max_x_err, max_y_err, max_z_err
    );

    let compressed = lepcc::api::compress_xyz(&points, max_x_err, max_y_err, max_z_err)?;

    let output_file = "arcgis_reencoded_rust.pccxyz";
    let file = File::create(output_file)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(&compressed)?;
    drop(writer);

    println!(
        "Compressed to {} bytes ({} bytes/point)",
        compressed.len(),
        compressed.len() as f64 / points.len() as f64
    );
    println!("Wrote to {}", output_file);

    // Print hex of first 64 bytes
    println!("\nFirst 64 bytes:");
    for i in (0..64usize).step_by(16) {
        let end = (i + 16).min(compressed.len());
        print!("{:04x}:", i);
        for j in i..end {
            print!(" {:02x}", compressed[j]);
        }
        println!();
    }

    Ok(())
}
