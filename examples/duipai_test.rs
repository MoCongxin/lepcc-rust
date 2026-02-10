// Rust对拍测试程序

use std::env;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::Path;

// 解析点数据文件
fn read_point_file(filename: &str) -> io::Result<(Vec<[f64; 3]>, u32)> {
    let file = File::open(filename)?;
    let mut reader = BufReader::new(file);

    // 读取点数 (4字节小端)
    let mut num_bytes = [0u8; 4];
    reader.read_exact(&mut num_bytes)?;
    let num_points = u32::from_le_bytes(num_bytes) as usize;

    println!("Reading {} points...", num_points);

    // 读取点数据
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

// 输出压缩数据
fn write_compressed_data(filename: &str, data: &[u8]) -> io::Result<()> {
    let file = File::create(filename)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(data)?;
    writer.flush()?;
    Ok(())
}

// 计算点之间的最大误差
fn calculate_max_errors(
    original: &[[f64; 3]],
    decoded: &[[f64; 3]],
    order: &[u32],
) -> (f64, f64, f64) {
    let mut max_err_x = 0.0;
    let mut max_err_y = 0.0;
    let mut max_err_z = 0.0;

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

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Usage: {} <input_bin_file>", args[0]);
        println!("Example: {} test_input.bin", args[0]);
        std::process::exit(1);
    }

    let input_file = &args[1];
    let output_file = "compressed_output_rust.bin";

    // 读取点数据
    println!("Reading input file: {}", input_file);
    let (points, num_points) = match read_point_file(input_file) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error reading input file: {}", e);
            std::process::exit(1);
        }
    };

    println!("Loaded {} points", num_points);

    // 显示前几个点
    println!("First 3 points:");
    for i in 0..std::cmp::min(3, points.len()) {
        println!(
            "  [{}] ({:.6}, {:.6}, {:.6})",
            i, points[i][0], points[i][1], points[i][2]
        );
    }

    // TODO: 实际的压缩逻辑需要在lepcc-rust中实现
    println!("\nNOTE: Rust压缩功能需要通过lepcc库实现");
    println!("Placeholder: This would call lepcc::compress_xyz()");

    // 解码验证
    println!("\nDecoding for verification...");
    // TODO: 实现解码逻辑

    println!("Max errors: would be calculated here");

    println!("\nNote: Full implementation needs lepcc-rust library");
}
