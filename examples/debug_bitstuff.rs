// Debug test for BitStuffer2 bit manipulation
use lepcc::bit_stuffer2::BitStuffer2;

fn main() {
    // Test with sequential data 0-35 to catch the corruption at index 31
    let data: Vec<u32> = (0..36).collect();
    println!("Input: {:?}", data);

    let encoded = BitStuffer2::encode_simple(&data).unwrap();
    println!("Encoded {} bytes: {:02X?}", encoded.len(), &encoded[..20]);

    // Decode with manual inspection
    let decoded = BitStuffer2::decode(&encoded).unwrap();
    println!("Decoded: {:?}", decoded);

    // Find mismatches
    for (i, (&a, &b)) in data.iter().zip(decoded.iter()).enumerate() {
        if a != b {
            println!(
                "Mismatch at index {}: expected {}, got {} (diff: {})",
                i,
                a,
                b,
                b as i32 - a as i32
            );
            println!("  Expected: {:07b}, Got: {:07b}", a, b);
        }
    }
}
