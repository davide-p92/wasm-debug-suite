
// src/hex_dump.rs
use std::fmt;

/// Converts a byte slice into a formatted hex dump string.
/// Each line shows the offset and 16 bytes in hex.
pub fn wasm_to_hex(bytes: &[u8]) -> String {
    let mut output = String::new();

    for (i, chunk) in bytes.chunks(16).enumerate() {
        // Offset in hex
        output.push_str(&format!("{:04X}: ", i * 16));

        // Hex representation
        for byte in chunk {
            output.push_str(&format!("{:02X} ", byte));
        }

        // Fill remaining spaces if chunk < 16
        if chunk.len() < 16 {
            output.push_str(&"   ".repeat(16 - chunk.len()));
        }

        // ASCII representation
        output.push_str(" |");
        for byte in chunk {
            let ch = if byte.is_ascii_graphic() { *byte as char } else { '.' };
            output.push(ch);
        }
        output.push_str("|\n");
    }

    output
}

