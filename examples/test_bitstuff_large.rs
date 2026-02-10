// Test BitStuffer2 with larger datasets

use lepcc::bit_stuffer2::BitStuffer2;

fn test_roundtrip(name: &str, data: &[u32]) {
    println!("\n=== Test: {} ===", name);
    println!("Input: {:?}", &data[..std::cmp::min(10, data.len())]);

    let encoded = BitStuffer2::encode_simple(data).unwrap();
    println!("Encoded size: {} bytes", encoded.len());

    let decoded = BitStuffer2::decode(&encoded).unwrap();
    println!(
        "Decoded: {:?}",
        &decoded[..std::cmp::min(10, decoded.len())]
    );

    if decoded == data {
        println!("✓ PASS");
    } else {
        println!("✗ FAIL");
        println!("  Expected: {:?}", data);
        println!("  Got:      {:?}", decoded);

        // Find first mismatch
        for (i, (a, b)) in data.iter().zip(decoded.iter()).enumerate() {
            if a != b {
                println!("  First mismatch at index {}: expected {}, got {}", i, a, b);
                break;
            }
        }
    }
}

fn main() {
    // Test 1: Sequential data
    let seq_data: Vec<u32> = (0..100).collect();
    test_roundtrip("Sequential 0-99", &seq_data);

    // Test 2: Repeated values
    let repeat_data = vec![5u32; 100];
    test_roundtrip("Repeated 5s (100)", &repeat_data);

    // Test 3: Alternating
    let alt_data: Vec<u32> = (0..100).map(|i| if i % 2 == 0 { 0 } else { 100 }).collect();
    test_roundtrip("Alternating 0/100", &alt_data);

    // Test 4: Large range
    let large_data: Vec<u32> = (0..100).map(|i| i * 1000).collect();
    test_roundtrip("Large range (0-99000)", &large_data);

    // Test 5: Bunny-like x_delta pattern (small values with occasional large jumps)
    let mut bunny_like = vec![0u32; 2503];
    for i in 0..bunny_like.len() {
        bunny_like[i] = (i % 10) as u32;
    }
    test_roundtrip("Bunny-like pattern", &bunny_like);

    // Test 6: Many zeros with occasional values
    let mut sparse = vec![0u32; 2503];
    for i in (0..sparse.len()).step_by(10) {
        sparse[i] = 50;
    }
    test_roundtrip("Sparse 50s in zeros", &sparse);

    println!("\n=== All tests completed ===");
}
