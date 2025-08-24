#![allow(dead_code)]

// TODO: refactor test cases to extract common setup code

const TMPDIR: &str = "../playground";
const DUMP_DIR: &str = "../playground/dump";

pub fn redirect_temp_dir() {
    unsafe {
        std::env::set_var("TMPDIR", TMPDIR);
    }

    // Ensure the TMPDIR exists
    if !std::path::Path::new(TMPDIR).exists() {
        std::fs::create_dir_all(TMPDIR).expect("Failed to create TMPDIR");
    }
}

// Helper function to create hexdump representation
pub fn to_hexdump(data: &[u8]) -> String {
    let mut result = String::new();
    for (i, chunk) in data.chunks(16).enumerate() {
        result.push_str(&format!("{:08x}: ", i * 16));

        // Hex representation
        for (j, byte) in chunk.iter().enumerate() {
            if j == 8 {
                result.push(' ');
            }
            result.push_str(&format!("{:02x} ", byte));
        }

        // Pad if less than 16 bytes
        for j in chunk.len()..16 {
            if j == 8 {
                result.push(' ');
            }
            result.push_str("   ");
        }

        result.push_str(" |");

        // ASCII representation
        for byte in chunk {
            if byte.is_ascii_graphic() {
                result.push(*byte as char);
            } else {
                result.push('.');
            }
        }

        result.push_str("|\n");
    }
    result
}

// Macro to compare index contents with hexdump output on failure
#[macro_export]
macro_rules! assert_index_eq {
    ($bit_content:expr, $git_content:expr) => {
        if $bit_content != $git_content {
            let bit_hexdump = common::to_hexdump($bit_content);
            let git_hexdump = common::to_hexdump($git_content);

            // Use pretty_assertions for better diff visualization
            pretty_assertions::assert_eq!(
                bit_hexdump,
                git_hexdump,
                "\n=== INDEX CONTENTS DIFFER ===\nBit index ({} bytes) vs Git index ({} bytes)",
                $bit_content.len(),
                $git_content.len()
            );
        }
    };
    ($bit_content:expr, $git_content:expr, $($arg:tt)*) => {
        if $bit_content != $git_content {
            let bit_hexdump = common::to_hexdump($bit_content);
            let git_hexdump = common::to_hexdump($git_content);

            // Use pretty_assertions for better diff visualization with custom message
            pretty_assertions::assert_eq!(
                bit_hexdump,
                git_hexdump,
                "\n=== INDEX CONTENTS DIFFER ===\n{}\nBit index ({} bytes) vs Git index ({} bytes)",
                format_args!($($arg)*),
                $bit_content.len(),
                $git_content.len()
            );
        }
    };
}
