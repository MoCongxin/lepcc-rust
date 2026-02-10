// Basic example demonstrating LEPCC compression

use lepcc::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("LEPCC Rust Implementation - Example");
    println!("===================================\n");

    // Example: Compress RGB colors
    println!("Example: RGB Compression");
    let colors = vec![
        RGB { r: 255, g: 0, b: 0 },
        RGB { r: 0, g: 255, b: 0 },
        RGB { r: 0, g: 0, b: 255 },
        RGB { r: 128, g: 128, b: 128 },
        RGB { r: 255, g: 255, b: 255 },
    ];
    println!("  Original colors: {}", colors.len());
    println!("  Original size: {} bytes", colors.len() * 3);

    let compressed = compress_rgb(&colors)?;
    println!("  Compressed size: {} bytes", compressed.len());

    let decompressed = decompress_rgb(&compressed)?;
    println!("  Decompressed colors: {}", decompressed.len());
    println!("  All colors preserved: {}\n",
             colors.iter().zip(decompressed.iter()).all(|(a, b)| a == b));

    println!("\nRGB compression test completed successfully!");

    Ok(())
}
