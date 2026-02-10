// Test BitStuffer2 encoding/decoding

use lepcc::bit_stuffer2::BitStuffer2;

fn main() {
    let test_data = vec![1u32, 2, 3, 5, 8, 13, 21, 34, 55, 89];

    println!("Original data: {:?}", test_data);

    // Encode
    let encoded = BitStuffer2::encode_simple(&test_data).unwrap();
    println!("Encoded size: {} bytes", encoded.len());
    println!("Encoded hex: {:02X?}", encoded);

    // Decode
    let decoded = BitStuffer2::decode(&encoded).unwrap();
    println!("Decoded data: {:?}", decoded);

    // Verify
    if decoded == test_data {
        println!("✓ Roundtrip successful!");
    } else {
        println!("✗ Roundtrip failed!");
        println!("  Expected: {:?}", test_data);
        println!("  Got:      {:?}", decoded);
    }

    // Test with zeros
    println!("\n--- Test with zeros ---");
    let zero_data = vec![0u32; 10];
    let encoded_zero = BitStuffer2::encode_simple(&zero_data).unwrap();
    println!("Zeros encoded size: {} bytes", encoded_zero.len());
    println!("Zeros encoded hex: {:02X?}", encoded_zero);

    let decoded_zero = BitStuffer2::decode(&encoded_zero).unwrap();
    if decoded_zero == zero_data {
        println!("✓ Zeros roundtrip successful!");
    } else {
        println!("✗ Zeros roundtrip failed!");
    }

    // Test with larger numbers
    println!("\n--- Test with larger numbers ---");
    let large_data = vec![100u32, 200, 300, 400, 500, 600, 700, 800, 900, 1000];
    let encoded_large = BitStuffer2::encode_simple(&large_data).unwrap();
    println!("Large encoded size: {} bytes", encoded_large.len());

    let decoded_large = BitStuffer2::decode(&encoded_large).unwrap();
    if decoded_large == large_data {
        println!("✓ Large roundtrip successful!");
    } else {
        println!("✗ Large roundtrip failed!");
        println!("  Expected: {:?}", large_data);
        println!("  Got:      {:?}", decoded_large);
    }
}
