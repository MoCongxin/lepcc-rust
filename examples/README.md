# LEPCC Examples

This directory contains example programs demonstrating how to use the LEPCC library for point cloud compression and decompression.

## Available Examples

### 1. encode_bunny.rs

**Purpose**: Encode the Stanford Bunny model to LEPCC format.

**Description**:
- Reads the Stanford Bunny OBJ file
- Compresses the 3D coordinates using LEPCC
- Saves the output to a `.pccxyz` file
- Demonstrates proper error handling

**Run**:
```bash
cargo run --release --example encode_bunny
```

**Output**:
```
Reading OBJ file: bunny.obj
Loaded 2503 points

Bounding box:
  X: [-0.09, 0.06] (span: 0.16)
  Y: [0.03, 0.19] (span: 0.15)
  Z: [-0.06, 0.06] (span: 0.12)

Compression tolerances:
  MaxErrorX: 0.0000016
  MaxErrorY: 0.0000015
  MaxErrorZ: 0.001

Compressed 2503 points into 9561 bytes
Wrote to bunny_encoded.pccxyz
```

---

### 2. decode_only.rs

**Purpose**: Decode a LEPCC compressed file and display point information.

**Description**:
- Reads a `.pccxyz` or `.pccrgb` file
- Detects the blob type automatically
- Decompresses the data
- Prints statistics about the point cloud

**Run**:
```bash
# Decode XYZ file
cargo run --release --example decode_only -- input.pccxyz

# Decode RGB file
cargo run --release --example decode_only -- input.pccrgb
```

**Output**:
```
Reading input.pccxyz...
File type: XYZ (LEPCC)
Blob size: 9561 bytes
Point count: 2503

Bounding box:
  X: [-0.09, 0.06]
  Y: [0.03, 0.19]
  Z: [-0.06, 0.06]

First 5 points:
  [0]: (-0.094, 0.033, -0.062)
  [1]: (-0.067, 0.037, -0.062)
  [2]: (-0.064, 0.043, -0.062)
  ...
```

---

### 3. duipai.rs

**Purpose**: Compare C++ and Rust output for verification (duipai = 对比 in Chinese).

**Description**:
- Encodes the same data with both C++ and Rust
- Compares the compressed output byte-by-byte
- Useful for validating the Rust implementation

**Run**:
```bash
cargo run --release --example duipai
```

**Prerequisites**:
- You must have the C++ LEPCC encoder compiled as a separate executable

**Output**:
```
Encoding with C++...
C++ output: 9561 bytes

Encoding with Rust...
Rust output: 9561 bytes

Comparing outputs...
✓ Compressed data is IDENTICAL (bytes 104-9561)
✓ Only checksum differs (known issue)
```

---

### 4. encode_only.rs

**Purpose**: Simple compression example with custom parameters.

**Description**:
- Demonstrates how to adjust compression quality
- Shows effect of different error tolerances on compression ratio

**Run**:
```bash
cargo run --release --example encode_only -- input.obj
```

---

### 5. encode_obj.rs

**Purpose**: Encode an OBJ file to LEPCC format with custom error parameters.

**Description**:
- Reads OBJ file
- Accepts custom error tolerances as command-line arguments
- Outputs compressed PCCXYZ file

**Run**:
```bash
cargo run --release --example encode_obj -- input.obj 0.01 0.01 0.01
# Arguments: input.obj max_x_err max_y_err max_z_err
```

---

### 6. encode_obj_exact.rs

**Purpose**: Encode with EXACT C++ max_error computation for compatibility.

**Description**:
- Computes max_error exactly like the C++ encoder
- Uses extent-based formula: `extent * 0.00001` with minimum threshold
- Ensures bit-identical output to C++

**Run**:
```bash
cargo run --release --example encode_obj_exact
```

---

### 7. encode_bunny_final.rs

**Purpose**: Encode bunny with precise error calculation.

**Description**:
- Demonstrates proper max_error computation
- Shows bounding box calculation
- Suitable as a reference for custom encoding

**Run**:
```bash
cargo run --release --example encode_bunny_final
```

---

### 8. encode_bunny_scaled.rs

**Purpose**: Encode scaled bunny model (100x larger).

**Description**:
- Uses pre-scaled bunny coordinates
- Demonstrates encoding large models
- Useful for ArcGIS testing (visibility)

**Run**:
```bash
cargo run --release --example encode_bunny_scaled
```

---

## Building All Examples

```bash
# Debug mode (faster compile, slower execution)
cargo build --examples

# Release mode (slower compile, faster execution)
cargo build --release --examples

# Optimize for size
cargo build --release --example <name> -- --release
```

## Running Examples

```bash
# Basic run (debug mode)
cargo run --example encode_bunny

# Release mode (recommended for performance testing)
cargo run --release --example encode_bunny

# With custom arguments
cargo run --release --example decode_only -- input.pccxyz

# Redirect output to file
cargo run --release --example encode_bunny > output.log 2>&1
```

## Using Examples in Your Code

You can copy and adapt example code for your projects:

```rust
use lepcc::prelude::*;
use std::fs;

// From encode_bunny.rs
fn encode_file(input_path: &str, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let points = read_obj_file(input_path)?;
    let max_err = 0.01; // 1cm tolerance
    let compressed = compress_xyz(&points, max_err, max_err, max_err)?;
    fs::write(output_path, compressed)?;
    println!("Compressed {} points to {}", points.len(), output_path);
    Ok(())
}

fn read_obj_file(path: &str) -> Result<Vec<Point3D>, Box<dyn std::error::Error>> {
    // ... implementation from examples ...
    Ok(vec![])
}
```

## Testing with Real Data

To test with your own point cloud data:

1. **Convert to OBJ format**:
   ```bash
   # If you have XYZ CSV:
   awk '{print "v", $1, $2, $3}' input.csv > input.obj
   ```

2. **Encode**:
   ```bash
   cargo run --release --example encode_obj -- input.obj 0.01 0.01 0.01
   ```

3. **Verify**:
   ```bash
   cargo run --release --example decode_only -- input.pccxyz
   ```

4. **Check compression ratio**:
   ```rust
   let original_size = points.len() * 24; // 3 doubles per point
   let compressed_len = compressed.len();
   let ratio = original_size as f64 / compressed_len as f64;
   println!("Compression ratio: {:.1}:1", ratio);
   ```

## Performance Testing

To benchmark compression speed:

```bash
# Install hyperfine for timing
cargo install hyperfine

# Run benchmark
hyperfine 'cargo run --release --example encode_bunny'
```

To benchmark with large datasets, modify examples to accept file paths:

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let input_path = if args.len() > 1 {
        &args[1]
    } else {
        "large_dataset.obj"
    };

    let start = std::time::Instant::now();
    let points = read_obj_file(input_path)?;
    let compressed = compress_xyz(&points, 0.01, 0.01, 0.01)?;
    let duration = start.elapsed();

    println!("Encoded {} points in {:?}", points.len(), duration);
    println!("Speed: {:.1} points/sec",
        points.len() as f64 / duration.as_secs_f64());

    Ok(())
}
```

## Common Issues and Solutions

### Issue: File not found

```bash
# Error: No such file or directory (os error 2)

# Solution: Use absolute path or run from correct directory
cargo run --release --example encode_bunny -- /full/path/to/bunny.obj
```

### Issue: Insufficient memory for large datasets

```rust
// Modify example to process in chunks
fn process_large_file(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut batch = Vec::with_capacity(100000); // 100K points per batch

    for line in reader.lines() {
        let line = line?;
        if line.starts_with("v ") {
            batch.push(parse_vertex(&line)?);
            if batch.len() >= 100000 {
                compress_batch(&batch)?;
                batch.clear();
            }
        }
    }

    if !batch.is_empty() {
        compress_batch(&batch)?;
    }

    Ok(())
}
```

### Issue: Compression too slow

```rust
// Reduce max_err for faster compression (lower quality)
let max_err = 0.1;  // 10cm tolerance (much faster than 1cm)

// Or process in parallel using rayon
use rayon::prelude::*;

points.par_chunks(10000)
    .for_each(|chunk| {
        let compressed = compress_xyz(chunk, 0.01, 0.01, 0.01).unwrap();
        // write to file...
    });
```

## Contributing Examples

To add a new example:

1. Create a new file in `examples/` directory
2. Add relevant explanation to this README
3. Ensure it compiles: `cargo build --example <name>`
4. Run it to verify: `cargo run --release --example <name>`
5. Update this file with example details

Example template:
```rust
//! [Brief description]
//!
//! [Detailed description]
//!
//! Usage:
//!     cargo run --release --example <name> [arguments]

use lepcc::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Your code here
    Ok(())
}
```

## Additional Resources

- [Main README](../README.md) - Library documentation
- [API Documentation](../src/api.rs) - API reference
- [LEPCC Encoder README](../LEPCC_ENCODER_README.md) - C++ encoder documentation
- [Esri LEPCC GitHub](https://github.com/Esri/lepcc) - Original C++ implementation
