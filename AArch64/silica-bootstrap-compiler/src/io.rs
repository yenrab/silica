/*
 * Silica I/O Runtime Functions
 *
 * Provides input/output functionality for the Silica programming language.
 * These functions are called from generated LLVM IR code.
 */

use std::io::Write;

/// Print a string to stdout
#[no_mangle]
pub extern "C" fn silica_print(str_ptr: *const u8, len: usize) {
    if !str_ptr.is_null() && len > 0 {
        let slice = unsafe { std::slice::from_raw_parts(str_ptr, len) };
        if let Ok(s) = std::str::from_utf8(slice) {
            print!("{}", s);
        }
        // Ensure output is flushed
        let _ = std::io::stdout().flush();
    }
}

/// Print a string followed by a newline to stdout
#[no_mangle]
pub extern "C" fn silica_println(str_ptr: *const u8, len: usize) {
    silica_print(str_ptr, len);
    println!();
}

/// Print a 64-bit integer to stdout
#[no_mangle]
pub extern "C" fn silica_print_int(n: i64) {
    print!("{}", n);
    let _ = std::io::stdout().flush();
}

/// Print a boolean value to stdout
#[no_mangle]
pub extern "C" fn silica_print_bool(b: bool) {
    print!("{}", if b { "true" } else { "false" });
    let _ = std::io::stdout().flush();
}

/// Print a single Unicode character to stdout
#[no_mangle]
pub extern "C" fn silica_print_char(c: u32) {
    if let Some(ch) = char::from_u32(c) {
        print!("{}", ch);
    } else {
        print!("�"); // Replacement character for invalid codepoints
    }
    let _ = std::io::stdout().flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print_functions() {
        // Note: These tests would require capturing stdout to verify output
        // For now, we just ensure the functions don't panic with valid inputs

        let test_str = b"Hello, World!";
        silica_print(test_str.as_ptr(), test_str.len());

        silica_println(test_str.as_ptr(), test_str.len());

        silica_print_int(42);
        silica_print_int(-123);

        silica_print_bool(true);
        silica_print_bool(false);

        silica_print_char(b'A');
        silica_print_char(b'!');
    }
}
