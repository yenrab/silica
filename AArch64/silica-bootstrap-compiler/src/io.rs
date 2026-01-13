/*
 * Silica I/O Runtime Functions
 *
 * Provides input/output functionality for the Silica programming language.
 * These functions are called from generated LLVM IR code.
 * Print operations use Rust's standard print functions directly.
 */

use std::io::Write;

/// SilicaString structure matching runtime.rs
#[repr(C)]
pub struct SilicaString {
    pub data: *mut u8,
    pub length: usize,
}

/// Print a string to stdout
#[no_mangle]
pub extern "C" fn silica_print(str_ptr: *const u8, len: usize) {
    if str_ptr.is_null() || len == 0 {
        return;
    }

    unsafe {
        let slice = std::slice::from_raw_parts(str_ptr, len);
        if let Ok(s) = std::str::from_utf8(slice) {
            print!("{}", s);
            let _ = std::io::stdout().flush();
        }
    }
}

/// Print a string followed by a newline to stdout
#[no_mangle]
pub extern "C" fn silica_println(str_ptr: *const u8, len: usize) {
    if str_ptr.is_null() || len == 0 {
        println!();
        return;
    }

    unsafe {
        let slice = std::slice::from_raw_parts(str_ptr, len);
        if let Ok(s) = std::str::from_utf8(slice) {
            println!("{}", s);
        } else {
            println!();
        }
    }
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

        silica_print_char(b'A' as u32);
        silica_print_char(b'!' as u32);
    }
}

/// Get the byte length of a SilicaString
/// Takes a pointer to a SilicaString struct and returns its byte length field
#[no_mangle]
pub extern "C" fn silica_string_len(silica_string_ptr: *const u8) -> usize {
    if silica_string_ptr.is_null() {
        return 0;
    }

    unsafe {
        // SilicaString is { data: *mut u8, length: usize }
        // Cast to pointer to usize to access the length field (second field)
        let ptr = silica_string_ptr as *const usize;
        let length_ptr = ptr.add(1); // length is at offset 1 (after data pointer)
        *length_ptr
    }
}

/// Get the character count of a SilicaString
/// Takes a pointer to a SilicaString struct and returns the number of Unicode characters
#[no_mangle]
pub extern "C" fn silica_string_len_chars(silica_string_ptr: *const u8) -> usize {
    if silica_string_ptr.is_null() {
        return 0;
    }

    unsafe {
        // SilicaString is { data: *mut u8, length: usize }
        // Read the struct fields
        let ptr = silica_string_ptr as *const usize;
        let data_ptr = *ptr as *const u8; // First field: data pointer
        let byte_length = *(ptr.add(1)); // Second field: byte length

        if data_ptr.is_null() || byte_length == 0 {
            return 0;
        }

        // Create a slice from the raw pointer and length
        let slice = std::slice::from_raw_parts(data_ptr, byte_length);
        
        // Convert to &str and count characters
        match std::str::from_utf8(slice) {
            Ok(s) => s.chars().count(),
            Err(_) => {
                // Invalid UTF-8 - count valid UTF-8 sequences
                // This is a fallback for malformed strings
                let mut count = 0;
                let mut i = 0;
                while i < byte_length {
                    // Try to decode a UTF-8 character
                    if let Some((_, len)) = std::str::from_utf8(&slice[i..]).ok()
                        .and_then(|s| s.chars().next().map(|c| (c, c.len_utf8()))) {
                        count += 1;
                        i += len;
                    } else {
                        // Skip invalid byte
                        i += 1;
                    }
                }
                count
            }
        }
    }
}

/// Helper to extract string data and length from either a string constant pointer or SilicaString pointer
unsafe fn get_string_data_and_length(ptr: *const u8) -> Option<(*const u8, usize)> {
    if ptr.is_null() {
        return None;
    }

    // Try to interpret as SilicaString pointer first
    // SilicaString is { data: *mut u8, length: usize }
    let silica_string_ptr = ptr as *const usize;
    let data_ptr = *silica_string_ptr as *const u8;
    let length = *(silica_string_ptr.add(1));

    // Heuristic: if data_ptr is not null and length is reasonable, assume it's a SilicaString
    if !data_ptr.is_null() && length < 1024 * 1024 * 1024 {
        // Verify the data pointer is valid by checking if it's different from the struct pointer
        if data_ptr != ptr {
            return Some((data_ptr, length));
        }
    }

    // Otherwise, treat as a null-terminated C string
    let mut len = 0;
    let mut p = ptr;
    while *p != 0 {
        len += 1;
        p = p.add(1);
        if len > 1024 * 1024 {
            // Safety limit
            break;
        }
    }
    Some((ptr, len))
}

/// Concatenate two strings
/// Accepts either string constant pointers (i8* to string data) or SilicaString pointers (i8* to SilicaString struct)
/// Returns i8* pointer to a new SilicaString struct containing the concatenated result
#[no_mangle]
pub extern "C" fn silica_string_concat(a_ptr: *const u8, b_ptr: *const u8) -> *mut u8 {
    if a_ptr.is_null() && b_ptr.is_null() {
        // Both null - return empty string
        return create_empty_silica_string();
    }

    // Get string data and lengths
    let (a_data, a_len) = unsafe { get_string_data_and_length(a_ptr).unwrap_or((std::ptr::null(), 0)) };
    let (b_data, b_len) = unsafe { get_string_data_and_length(b_ptr).unwrap_or((std::ptr::null(), 0)) };

    // Calculate total length
    let total_len = a_len + b_len;

    if total_len == 0 {
        return create_empty_silica_string();
    }

    // Allocate buffer for concatenated string
    let mut buffer = Vec::with_capacity(total_len);
    
    // Copy first string
    if a_len > 0 && !a_data.is_null() {
        unsafe {
            let slice = std::slice::from_raw_parts(a_data, a_len);
            buffer.extend_from_slice(slice);
        }
    }

    // Copy second string
    if b_len > 0 && !b_data.is_null() {
        unsafe {
            let slice = std::slice::from_raw_parts(b_data, b_len);
            buffer.extend_from_slice(slice);
        }
    }

    // Create SilicaString from the buffer
    create_silica_string_from_bytes(&buffer)
}

/// Create an empty SilicaString
fn create_empty_silica_string() -> *mut u8 {
    let empty: Vec<u8> = Vec::new();
    create_silica_string_from_bytes(&empty)
}

/// Create a SilicaString from a byte slice
fn create_silica_string_from_bytes(bytes: &[u8]) -> *mut u8 {
    // Allocate memory for the string data
    let data_ptr = if bytes.is_empty() {
        std::ptr::null_mut()
    } else {
        let mut data = Vec::with_capacity(bytes.len());
        data.extend_from_slice(bytes);
        let ptr = data.as_mut_ptr();
        std::mem::forget(data); // Leak the Vec to keep the data alive
        ptr
    };

    // Create SilicaString struct
    let silica_string = Box::new(SilicaString {
        data: data_ptr,
        length: bytes.len(),
    });

    // Return pointer to SilicaString (cast to i8* for C compatibility)
    Box::into_raw(silica_string) as *mut u8
}
