// 简单的对拍测试程序

use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::Path;

fn read_point_file(filename: &str) -> io::Result<(Vec<[f64; 3]>, u32)> {
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

    Ok((points, num_points as u32))
}

fn calculate_max_errors(
    original: &[[f64; 3]],
    decoded: &[[f64; 3]],
    order: &[u32],
) -> (f64, f64, f64) {
    let mut max_err_x: f64 = 0.0;
    let mut max_err_y: f64 = 0.0;
    let mut max_err_z: f64 = 0.0;

    for (i, &orig_idx) in order.iter().enumerate() {
        let orig = original[orig_idx as usize];
        let dec = decoded[i];

        let err_x = (orig[0] - dec[0]).abs();
        let err_y = (orig[1] - dec[1]).abs();
        let err_z = (orig[2] - dec[2]).abs();

        max_err_x = max_err_x.max(err_x);
        max_err_y = max_err_y.max(err_y);
        max_err_z = max_err_z.max(err_z);
    }

    (max_err_x, max_err_y, max_err_z)
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        println!("Usage: {} <input_bin_file> <mode>", args[0]);
        println!("  mode: 'encode', 'decode', or 'test'");
        println!("\nExample:");
        println!("  {} test_input.bin test", args[0]);
        println!("  {} test_input.bin encode", args[0]);
        std::process::exit(1);
    }

    let input_file = &args[1];
    let mode = &args[2];

    match mode.as_str() {
        "encode" => {
            let (points, _) = read_point_file(input_file)?;
            println!("Encoding {} points...", points.len());

            // 转换为Point3D
            let point3d_vec: Vec<lepcc::Point3D> = points
                .iter()
                .map(|p| lepcc::Point3D::new(p[0], p[1], p[2]))
                .collect();

            // 编码
            let compressed =
                lepcc::api::compress_xyz(&point3d_vec, 0.01, 0.01, 0.01).expect("Encoding failed");

            let output_file = "compressed_output_rust.bin";
            let file = File::create(output_file)?;
            let mut writer = BufWriter::new(file);
            writer.write_all(&compressed)?;

            println!("Encoded to {} ({} bytes)", output_file, compressed.len());
        }
        "decode" => {
            let mut file = File::open(input_file)?;
            let mut compressed_data = Vec::new();
            file.read_to_end(&mut compressed_data)?;

            println!("Decoding {} bytes...", compressed_data.len());

            // 解码
            let decoded = lepcc::api::decompress_xyz(&compressed_data).expect("Decoding failed");

            println!("Decoded {} points", decoded.len());

            let output_file = "decoded_output_rust.bin";
            let file = File::create(output_file)?;
            let mut writer = BufWriter::new(file);

            // 写入点数
            writer.write_all(&(decoded.len() as u32).to_le_bytes())?;

            // 写入点数据
            for pt in decoded {
                writer.write_all(&pt.x.to_le_bytes())?;
                writer.write_all(&pt.y.to_le_bytes())?;
                writer.write_all(&pt.z.to_le_bytes())?;
            }

            println!("Decoded data written to {}", output_file);
        }
        "test" => {
            let (points, _) = read_point_file(input_file)?;
            println!("Testing compression roundtrip...");

            let point3d_vec: Vec<lepcc::Point3D> = points
                .iter()
                .map(|p| lepcc::Point3D::new(p[0], p[1], p[2]))
                .collect();

            // 编码
            let compressed =
                lepcc::api::compress_xyz(&point3d_vec, 0.01, 0.01, 0.01).expect("Encoding failed");

            println!(
                "Compressed to {} bytes (ratio: {:.2}x)",
                compressed.len(),
                (point3d_vec.len() * 24) as f64 / compressed.len() as f64
            );

            // 解码
            let decoded = lepcc::api::decompress_xyz(&compressed).expect("Decoding failed");

            println!("Decoded {} points", decoded.len());

            // 计算最大误差（简化版本 - 没有考虑原始顺序）
            let mut max_err_x: f64 = 0.0;
            let mut max_err_y: f64 = 0.0;
            let mut max_err_z: f64 = 0.0;

            for (orig, dec) in point3d_vec.iter().zip(decoded.iter()) {
                let err_x = (orig.x - dec.x).abs();
                let err_y = (orig.y - dec.y).abs();
                let err_z = (orig.z - dec.z).abs();

                max_err_x = max_err_x.max(err_x);
                max_err_y = max_err_y.max(err_y);
                max_err_z = max_err_z.max(err_z);
            }

            println!(
                "Max errors: X={:.6}, Y={:.6}, Z={:.6}",
                max_err_x, max_err_y, max_err_z
            );

            if max_err_x <= 0.01 && max_err_y <= 0.01 && max_err_z <= 0.01 {
                println!("✓ Test passed!");
            } else {
                println!("✗ Test failed: errors exceed tolerance");
            }
        }
        "decode" => {
            let mut file = File::open(input_file)?;
            let mut compressed_data = Vec::new();
            file.read_to_end(&mut compressed_data)?;

            println!("Decoding {} bytes...", compressed_data.len());

            // 解码
            let decoded = lepcc::api::decompress_xyz(&compressed_data).expect("Decoding failed");

            println!("Decoded {} points", decoded.len());

            let output_file = "decoded_output_rust.bin";
            let file = File::create(output_file)?;
            let mut writer = BufWriter::new(file);

            // 写入点数
            writer.write_all(&(decoded.len() as u32).to_le_bytes())?;

            // 写入点数据
            for pt in decoded {
                writer.write_all(&pt.x.to_le_bytes())?;
                writer.write_all(&pt.y.to_le_bytes())?;
                writer.write_all(&pt.z.to_le_bytes())?;
            }

            println!("Decoded data written to {}", output_file);
        }
        "test" => {
            let (points, _) = read_point_file(input_file)?;
            println!("Testing compression roundtrip...");

            let point3d_vec: Vec<lepcc::Point3D> = points
                .iter()
                .map(|p| lepcc::Point3D::new(p[0], p[1], p[2]))
                .collect();

            // 编码
            let compressed =
                lepcc::api::compress_xyz(&point3d_vec, 0.01, 0.01, 0.01).expect("Encoding failed");

            println!(
                "Compressed to {} bytes (ratio: {:.2}x)",
                compressed.len(),
                (point3d_vec.len() * 24) as f64 / compressed.len() as f64
            );

            // 解码
            let decoded = lepcc::api::decompress_xyz(&compressed).expect("Decoding failed");

            println!("Decoded {} points", decoded.len());

            // 计算最大误差（简化版本 - 没有考虑原始顺序）
            let mut max_err_x: f64 = 0.0;
            let mut max_err_y: f64 = 0.0;
            let mut max_err_z: f64 = 0.0;

            for (orig, dec) in point3d_vec.iter().zip(decoded.iter()) {
                let err_x = (orig.x - dec.x).abs();
                let err_y = (orig.y - dec.y).abs();
                let err_z = (orig.z - dec.z).abs();

                max_err_x = max_err_x.max(err_x);
                max_err_y = max_err_y.max(err_y);
                max_err_z = max_err_z.max(err_z);
            }

            println!(
                "Max errors: X={:.6}, Y={:.6}, Z={:.6}",
                max_err_x, max_err_y, max_err_z
            );

            if max_err_x <= 0.01 && max_err_y <= 0.01 && max_err_z <= 0.01 {
                println!("✓ Test passed!");
            } else {
                println!("✗ Test failed: errors exceed tolerance");
            }
        }
        _ => {
            println!("Unknown mode: {}", mode);
            println!("Valid modes: encode, decode, test");
            std::process::exit(1);
        }
    }

    Ok(())
}
