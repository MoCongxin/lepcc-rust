use lepcc::bit_stuffer2::BitStuffer2;

fn main() {
    // Test simple data that matches x_delta_vec section 1 data: 106 elements, num_bits=11
    let test_data: Vec<u32> = vec![
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
        48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70,
        71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93,
        94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105,
    ];

    let encoded = BitStuffer2::encode_simple(&test_data).expect("encode failed");
    println!(
        "Encoded {} elements with max value {}",
        test_data.len(),
        test_data.iter().max().unwrap()
    );
    println!("Output: {} bytes", encoded.len());
    println!(
        "First 20 bytes: {}",
        encoded[0..encoded.len().min(20)]
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join(" ")
    );

    // Decode and verify
    let decoded = BitStuffer2::decode(&encoded).expect("decode failed");
    println!("Decoded {} elements", decoded.len());
    println!("Matches original: {}", decoded == test_data);

    if decoded != test_data {
        println!("First 10 differences:");
        for i in 0..test_data.len().min(10) {
            if test_data[i] != decoded[i] {
                println!(
                    "  [{}] original={}, decoded={}",
                    i, test_data[i], decoded[i]
                );
            }
        }
    }
}
