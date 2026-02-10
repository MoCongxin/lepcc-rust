// Debug encoding with bunny

use lepcc::prelude::*;
use std::fs::File;
use std::io::{BufRead, BufReader};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("LEPCC_DEBUG", "1");

    // Read bunny.obj
    let obj_path = r"C:\Users\admin\Desktop\i3s文档\bunny.obj";
    let file = File::open(obj_path)?;
    let reader = BufReader::new(file);
    let mut points = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();
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

    println!("=== Test with {} points ===\n", points.len());
    for (i, p) in points.iter().enumerate() {
        println!("Original [{}]: ({:.6}, {:.6}, {:.6})", i, p.x, p.y, p.z);
    }

    let max_err = 0.01;
    println!("\nCompressing with max_err = {:.4}...", max_err);

    let compressed = compress_xyz(&points, max_err, max_err, max_err)?;
    println!("\nCompressed size: {} bytes", compressed.len());

    // Print header info
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

    let x_min = f64::from_le_bytes([
        compressed[24],
        compressed[25],
        compressed[26],
        compressed[27],
        compressed[28],
        compressed[29],
        compressed[30],
        compressed[31],
    ]);
    let y_min = f64::from_le_bytes([
        compressed[32],
        compressed[33],
        compressed[34],
        compressed[35],
        compressed[36],
        compressed[37],
        compressed[38],
        compressed[39],
    ]);
    let z_min = f64::from_le_bytes([
        compressed[40],
        compressed[41],
        compressed[42],
        compressed[43],
        compressed[44],
        compressed[45],
        compressed[46],
        compressed[47],
    ]);
    println!("Extent min: ({:.6}, {:.6}, {:.6})", x_min, y_min, z_min);

    let x_max = f64::from_le_bytes([
        compressed[48],
        compressed[49],
        compressed[50],
        compressed[51],
        compressed[52],
        compressed[53],
        compressed[54],
        compressed[55],
    ]);
    let y_max = f64::from_le_bytes([
        compressed[56],
        compressed[57],
        compressed[58],
        compressed[59],
        compressed[60],
        compressed[61],
        compressed[62],
        compressed[63],
    ]);
    let z_max = f64::from_le_bytes([
        compressed[64],
        compressed[65],
        compressed[66],
        compressed[67],
        compressed[68],
        compressed[69],
        compressed[70],
        compressed[71],
    ]);
    println!("Extent max: ({:.6}, {:.6}, {:.6})", x_max, y_max, z_max);

    let max_err_x = f64::from_le_bytes([
        compressed[72],
        compressed[73],
        compressed[74],
        compressed[75],
        compressed[76],
        compressed[77],
        compressed[78],
        compressed[79],
    ]);
    let max_err_y = f64::from_le_bytes([
        compressed[80],
        compressed[81],
        compressed[82],
        compressed[83],
        compressed[84],
        compressed[85],
        compressed[86],
        compressed[87],
    ]);
    let max_err_z = f64::from_le_bytes([
        compressed[88],
        compressed[89],
        compressed[90],
        compressed[91],
        compressed[92],
        compressed[93],
        compressed[94],
        compressed[95],
    ]);
    println!(
        "Max error: ({:.6}, {:.6}, {:.6})",
        max_err_x, max_err_y, max_err_z
    );

    let num_points = u32::from_le_bytes([
        compressed[96],
        compressed[97],
        compressed[98],
        compressed[99],
    ]);
    println!("Num points: {}", num_points);

    println!("\n=== Decoding ===");
    let decompressed = decompress_xyz(&compressed)?;

    println!("\n=== Comparison ===");
    let mut max_dx = 0.0f64;
    let mut max_dy = 0.0f64;
    let mut max_dz = 0.0f64;

    for (i, (orig, dec)) in points.iter().zip(decompressed.iter()).enumerate() {
        let dx = (orig.x - dec.x).abs();
        let dy = (orig.y - dec.y).abs();
        let dz = (orig.z - dec.z).abs();
        max_dx = max_dx.max(dx);
        max_dy = max_dy.max(dy);
        max_dz = max_dz.max(dz);

        println!("Point [{}]:", i);
        println!("  Original: ({:.6}, {:.6}, {:.6})", orig.x, orig.y, orig.z);
        println!("  Decoded:  ({:.6}, {:.6}, {:.6})", dec.x, dec.y, dec.z);
        println!("  Error:    ({:.6}, {:.6}, {:.6})", dx, dy, dz);
    }

    println!("\nMax error: ({:.6}, {:.6}, {:.6})", max_dx, max_dy, max_dz);
    println!(
        "Limit:     ({:.6}, {:.6}, {:.6})",
        max_err * 2.0,
        max_err * 2.0,
        max_err * 2.0
    );

    if max_dx <= max_err * 2.0 && max_dy <= max_err * 2.0 && max_dz <= max_err * 2.0 {
        println!("\n✓ PASS - All errors within limits!");
    } else {
        println!("\n✗ FAIL - Errors exceed limits!");
    }

    Ok(())
}
