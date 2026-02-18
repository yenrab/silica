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

/// Print a string to stdout (null-safe).
/// Accepts EITHER a SilicaString struct pointer OR a raw string constant pointer (i8* to char data).
/// Uses the same heuristic as get_string_data_and_length to handle both representations.
#[no_mangle]
pub extern "C" fn silica_print_string(ptr: *const u8) {
    if ptr.is_null() {
        return;
    }
    let (data_ptr, len) = unsafe { get_string_data_and_length(ptr).unwrap_or((std::ptr::null(), 0)) };
    if data_ptr.is_null() || len == 0 {
        return;
    }
    unsafe {
        let slice = std::slice::from_raw_parts(data_ptr, len);
        if let Ok(s) = std::str::from_utf8(slice) {
            print!("{}", s);
            let _ = std::io::stdout().flush();
        }
    }
}

/// Print a string to stdout followed by newline (null-safe).
/// Accepts EITHER a SilicaString struct pointer OR a raw string constant pointer (i8* to char data).
#[no_mangle]
pub extern "C" fn silica_println_string(ptr: *const u8) {
    if ptr.is_null() {
        println!();
        return;
    }
    let (data_ptr, len) = unsafe { get_string_data_and_length(ptr).unwrap_or((std::ptr::null(), 0)) };
    if data_ptr.is_null() || len == 0 {
        println!();
        return;
    }
    unsafe {
        let slice = std::slice::from_raw_parts(data_ptr, len);
        if let Ok(s) = std::str::from_utf8(slice) {
            println!("{}", s);
        } else {
            println!();
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
pub extern "C" fn silica_print_int64(n: i64) {
    print!("{}", n);
    let _ = std::io::stdout().flush();
}

/// Print an 8-bit integer to stdout
#[no_mangle]
pub extern "C" fn silica_print_int8(n: i8) {
    print!("{}", n);
    let _ = std::io::stdout().flush();
}

/// Print a 16-bit integer to stdout
#[no_mangle]
pub extern "C" fn silica_print_int16(n: i16) {
    print!("{}", n);
    let _ = std::io::stdout().flush();
}

/// Print a 32-bit integer to stdout
#[no_mangle]
pub extern "C" fn silica_print_int32(n: i32) {
    print!("{}", n);
    let _ = std::io::stdout().flush();
}

/// Print a boolean value to stdout
/// Receives i8 (0 or 1) for C ABI compatibility; i1 causes segfault when passed from LLVM
#[no_mangle]
pub extern "C" fn silica_print_bool(b: u8) {
    print!("{}", if b != 0 { "true" } else { "false" });
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

/// Print a float16 (half-precision) value to stdout
/// Receives the value as u16 (16 bits) and converts to f32 for display
#[no_mangle]
pub extern "C" fn silica_print_float16(half_bits: u16) {
    // Convert half-precision (16-bit) float to f32 for printing
    // IEEE 754 binary16 format: 1 sign bit, 5 exponent bits, 10 mantissa bits
    let f32_val = half_to_f32(half_bits);
    print!("{}", f32_val);
    let _ = std::io::stdout().flush();
}

/// Print a float32 (single-precision) value to stdout
#[no_mangle]
pub extern "C" fn silica_print_float32(value: f32) {
    print!("{}", value);
    let _ = std::io::stdout().flush();
}

/// Print a float64 (double-precision) value to stdout
#[no_mangle]
pub extern "C" fn silica_print_float64(value: f64) {
    print!("{}", value);
    let _ = std::io::stdout().flush();
}

/// Convert IEEE 754 binary16 (half-precision) to f32
fn half_to_f32(half: u16) -> f32 {
    // Extract sign, exponent, and mantissa
    let sign = (half >> 15) & 0x1;
    let exponent = (half >> 10) & 0x1F;
    let mantissa = half & 0x3FF;
    
    // Handle special cases
    if exponent == 0 {
        if mantissa == 0 {
            // Zero (positive or negative)
            if sign == 0 {
                0.0
            } else {
                -0.0
            }
        } else {
            // Denormalized number
            let val = (mantissa as f32) * 2.0_f32.powi(-24);
            if sign == 0 {
                val
            } else {
                -val
            }
        }
    } else if exponent == 0x1F {
        // Infinity or NaN
        if mantissa == 0 {
            if sign == 0 {
                f32::INFINITY
            } else {
                f32::NEG_INFINITY
            }
        } else {
            f32::NAN
        }
    } else {
        // Normalized number
        let exp = (exponent as i32) - 15 + 127; // Adjust bias from 15 to 127
        let mant = mantissa << 13; // Shift mantissa to f32 position (23 bits total, 10 from half)
        let bits = ((sign as u32) << 31) | ((exp as u32) << 23) | (mant as u32);
        f32::from_bits(bits)
    }
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

        silica_print_int64(42);
        silica_print_int64(-123);

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

/// Helper to extract string data and length from either a string constant pointer or SilicaString pointer.
/// Public for use by runtime.rs silica_read_file_path.
pub unsafe fn get_string_data_and_length(ptr: *const u8) -> Option<(*const u8, usize)> {
    // eprintln!("[DEBUG] get_string_data_and_length: ptr = {:p}", ptr);
    
    if ptr.is_null() {
        // eprintln!("[DEBUG] get_string_data_and_length: ptr is null, returning None");
        return None;
    }

    // Try to interpret as SilicaString pointer first
    // SilicaString is { data: *mut u8, length: usize }
    let silica_string_ptr = ptr as *const usize;
    // eprintln!("[DEBUG] get_string_data_and_length: silica_string_ptr = {:p}", silica_string_ptr);
    
    // Read the first field (data pointer)
    // eprintln!("[DEBUG] get_string_data_and_length: About to read data_ptr from struct...");
    let data_ptr = *silica_string_ptr as *const u8;
    // eprintln!("[DEBUG] get_string_data_and_length: data_ptr = {:p}", data_ptr);
    
    // Read the second field (length)
    // eprintln!("[DEBUG] get_string_data_and_length: About to read length from struct...");
    let length = *(silica_string_ptr.add(1));
    // eprintln!("[DEBUG] get_string_data_and_length: length = {}", length);

    // Heuristic: if data_ptr is not null and length is reasonable, assume it's a SilicaString
    // Also check that data_ptr is a reasonable pointer value (not too large, which would indicate
    // we're reading string data as if it were a pointer)
    let data_ptr_value = data_ptr as usize;
    let ptr_value = ptr as usize;
    
    // Check if data_ptr looks like a valid pointer:
    // - Not null
    // - Different from struct pointer
    // - Within reasonable memory range (typical user space addresses on 64-bit systems)
    // - Length is reasonable
    let looks_like_valid_pointer = !data_ptr.is_null() 
        && data_ptr != ptr
        && data_ptr_value < 0x7fffffffffff  // Reasonable upper bound for user space
        && data_ptr_value > 0x1000;  // Reasonable lower bound (avoid null page)
    
    if looks_like_valid_pointer && length < 1024 * 1024 * 1024 {
        return Some((data_ptr, length));
    }

    // Otherwise, treat as a null-terminated C string
    let mut len = 0;
    let mut p = ptr;
    while *p != 0 {
        len += 1;
        p = p.add(1);
        if len > 1024 * 1024 {
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

/// Extract a substring from a string
/// Accepts either string constant pointers (i8* to string data) or SilicaString pointers (i8* to SilicaString struct)
/// start and end are byte indices (inclusive start, exclusive end)
/// Returns i8* pointer to a new SilicaString struct containing the substring
#[no_mangle]
pub extern "C" fn silica_string_substring(str_ptr: *const u8, start: i64, end: i64) -> *mut u8 {
    if str_ptr.is_null() {
        return create_empty_silica_string();
    }

    // Get string data and length
    let (data, len) = unsafe { get_string_data_and_length(str_ptr).unwrap_or((std::ptr::null(), 0)) };
    
    if len == 0 {
        return create_empty_silica_string();
    }

    // Clamp indices to valid range
    let start_idx = start.max(0).min(len as i64) as usize;
    let end_idx = end.max(0).min(len as i64) as usize;
    
    // Ensure start <= end
    let start_idx = start_idx.min(end_idx);
    let end_idx = end_idx.max(start_idx);
    
    // Calculate substring length
    let sub_len = end_idx - start_idx;
    
    if sub_len == 0 {
        return create_empty_silica_string();
    }

    // Extract the substring bytes
    let mut buffer = Vec::with_capacity(sub_len);
    unsafe {
        let slice = std::slice::from_raw_parts(data.add(start_idx), sub_len);
        buffer.extend_from_slice(slice);
    }

    // Create SilicaString from the substring bytes
    create_silica_string_from_bytes(&buffer)
}

/// Extract a substring from a string until a specific character is found
/// Accepts either string constant pointers (i8* to string data) or SilicaString pointers (i8* to SilicaString struct)
/// start is the byte index to start from
/// char_code is the Unicode code point (i32) of the character to search for
/// Returns i8* pointer to a new SilicaString struct containing the substring (excluding the terminating character)
/// If the character is not found, returns the substring from start to the end of the string
#[no_mangle]
pub extern "C" fn silica_string_substring_until_char(str_ptr: *const u8, start: i64, char_code: i32) -> *mut u8 {
    if str_ptr.is_null() {
        return create_empty_silica_string();
    }

    // Get string data and length
    let (data, len) = unsafe { get_string_data_and_length(str_ptr).unwrap_or((std::ptr::null(), 0)) };
    
    if len == 0 {
        return create_empty_silica_string();
    }

    // Clamp start index to valid range
    let start_idx = start.max(0).min(len as i64) as usize;
    
    if start_idx >= len {
        return create_empty_silica_string();
    }

    // Convert char code to char (if valid)
    let target_char = match char::from_u32(char_code as u32) {
        Some(c) => c,
        None => {
            // Invalid character code - return empty string
            return create_empty_silica_string();
        }
    };

    // Search for the character starting from start_idx
    let mut end_idx = len;
    unsafe {
        let slice = std::slice::from_raw_parts(data.add(start_idx), len - start_idx);
        // Convert to string to search for character
        if let Ok(s) = std::str::from_utf8(slice) {
            // Search for the character in the string
            if let Some(pos) = s.chars().position(|c| c == target_char) {
                // Found the character - calculate byte position
                // We need to find the byte offset corresponding to the character position
                let mut byte_offset = 0;
                for (i, c) in s.chars().enumerate() {
                    if i == pos {
                        break;
                    }
                    byte_offset += c.len_utf8();
                }
                end_idx = start_idx + byte_offset;
            }
            // If not found, end_idx remains at len (return rest of string)
        } else {
            // Invalid UTF-8 - search byte by byte
            for i in start_idx..len {
                if slice[i - start_idx] == target_char as u8 {
                    end_idx = i;
                    break;
                }
            }
        }
    }

    // Extract the substring bytes
    let sub_len = end_idx - start_idx;
    
    if sub_len == 0 {
        return create_empty_silica_string();
    }

    let mut buffer = Vec::with_capacity(sub_len);
    unsafe {
        let slice = std::slice::from_raw_parts(data.add(start_idx), sub_len);
        buffer.extend_from_slice(slice);
    }

    // Create SilicaString from the substring bytes
    create_silica_string_from_bytes(&buffer)
}

/// Check if a string starts with a prefix
/// Accepts either string constant pointers (i8* to string data) or SilicaString pointers (i8* to SilicaString struct)
/// Returns true if the string starts with the prefix, false otherwise
#[no_mangle]
pub extern "C" fn silica_string_starts_with(str_ptr: *const u8, prefix_ptr: *const u8) -> bool {
    // eprintln!("[DEBUG] silica_string_starts_with: called with str_ptr={:p}, prefix_ptr={:p}", str_ptr, prefix_ptr);
    
    if str_ptr.is_null() || prefix_ptr.is_null() {
        // eprintln!("[DEBUG] silica_string_starts_with: One or both pointers are null, returning false");
        return false;
    }

    // Get string data and lengths
    // eprintln!("[DEBUG] silica_string_starts_with: Getting string data and length for str_ptr...");
    let (str_data, str_len) = unsafe { get_string_data_and_length(str_ptr).unwrap_or((std::ptr::null(), 0)) };
    // eprintln!("[DEBUG] silica_string_starts_with: str_data={:p}, str_len={}", str_data, str_len);
    
    // eprintln!("[DEBUG] silica_string_starts_with: Getting string data and length for prefix_ptr...");
    let (prefix_data, prefix_len) = unsafe { get_string_data_and_length(prefix_ptr).unwrap_or((std::ptr::null(), 0)) };
    // eprintln!("[DEBUG] silica_string_starts_with: prefix_data={:p}, prefix_len={}", prefix_data, prefix_len);

    // Empty prefix always matches
    if prefix_len == 0 {
        // eprintln!("[DEBUG] silica_string_starts_with: prefix_len is 0, returning true");
        return true;
    }

    // If prefix is longer than string, can't match
    if prefix_len > str_len {
        // eprintln!("[DEBUG] silica_string_starts_with: prefix_len ({}) > str_len ({}), returning false", prefix_len, str_len);
        return false;
    }

    // Compare bytes
    // eprintln!("[DEBUG] silica_string_starts_with: Comparing {} bytes...", prefix_len);
    unsafe {
        let str_slice = std::slice::from_raw_parts(str_data, prefix_len);
        let prefix_slice = std::slice::from_raw_parts(prefix_data, prefix_len);
        let result = str_slice == prefix_slice;
        // eprintln!("[DEBUG] silica_string_starts_with: Comparison result = {}", result);
        result
    }
}

/// Check if a string ends with a suffix
/// Accepts either string constant pointers (i8* to string data) or SilicaString pointers (i8* to SilicaString struct)
/// Returns true if the string ends with the suffix, false otherwise
#[no_mangle]
pub extern "C" fn silica_string_ends_with(str_ptr: *const u8, suffix_ptr: *const u8) -> bool {
    if str_ptr.is_null() || suffix_ptr.is_null() {
        return false;
    }

    // Get string data and lengths
    let (str_data, str_len) = unsafe { get_string_data_and_length(str_ptr).unwrap_or((std::ptr::null(), 0)) };
    let (suffix_data, suffix_len) = unsafe { get_string_data_and_length(suffix_ptr).unwrap_or((std::ptr::null(), 0)) };

    // Empty suffix always matches
    if suffix_len == 0 {
        return true;
    }

    // If suffix is longer than string, can't match
    if suffix_len > str_len {
        return false;
    }

    // Compare bytes from the end
    // Start comparing from position (str_len - suffix_len)
    unsafe {
        let str_slice = std::slice::from_raw_parts(str_data.add(str_len - suffix_len), suffix_len);
        let suffix_slice = std::slice::from_raw_parts(suffix_data, suffix_len);
        str_slice == suffix_slice
    }
}

/// Check if a string contains a substring
/// Accepts either string constant pointers (i8* to string data) or SilicaString pointers (i8* to SilicaString struct)
/// Returns true if the string contains the substring, false otherwise
#[no_mangle]
pub extern "C" fn silica_string_contains(str_ptr: *const u8, substr_ptr: *const u8) -> bool {
    if str_ptr.is_null() || substr_ptr.is_null() {
        return false;
    }

    // Get string data and lengths
    let (str_data, str_len) = unsafe { get_string_data_and_length(str_ptr).unwrap_or((std::ptr::null(), 0)) };
    let (substr_data, substr_len) = unsafe { get_string_data_and_length(substr_ptr).unwrap_or((std::ptr::null(), 0)) };

    // Empty substring always matches
    if substr_len == 0 {
        return true;
    }

    // If substring is longer than string, can't match
    if substr_len > str_len {
        return false;
    }

    // Search for the substring in the string
    // Use Rust's built-in string search for UTF-8 safety
    unsafe {
        let str_slice = std::slice::from_raw_parts(str_data, str_len);
        let substr_slice = std::slice::from_raw_parts(substr_data, substr_len);
        
        // Try UTF-8 string search first (more efficient and UTF-8 safe)
        if let (Ok(str_utf8), Ok(substr_utf8)) = (std::str::from_utf8(str_slice), std::str::from_utf8(substr_slice)) {
            return str_utf8.contains(substr_utf8);
        }
        
        // Fallback to byte search for invalid UTF-8
        // Simple linear search
        for i in 0..=(str_len - substr_len) {
            let candidate = std::slice::from_raw_parts(str_data.add(i), substr_len);
            if candidate == substr_slice {
                return true;
            }
        }
        
        false
    }
}

/// Check if two strings are equal.
/// Accepts either string constant pointers (i8* to string data) or SilicaString pointers (i8* to SilicaString struct).
/// Returns true if both strings have the same length and same bytes, false otherwise.
#[no_mangle]
pub extern "C" fn silica_string_equals(a_ptr: *const u8, b_ptr: *const u8) -> bool {
    if a_ptr.is_null() && b_ptr.is_null() {
        return true;
    }
    if a_ptr.is_null() || b_ptr.is_null() {
        return false;
    }

    let (a_data, a_len) = unsafe { get_string_data_and_length(a_ptr).unwrap_or((std::ptr::null(), 0)) };
    let (b_data, b_len) = unsafe { get_string_data_and_length(b_ptr).unwrap_or((std::ptr::null(), 0)) };

    if a_len != b_len {
        return false;
    }
    if a_len == 0 {
        return true;
    }

    let result = unsafe {
        let a_slice = std::slice::from_raw_parts(a_data, a_len);
        let b_slice = std::slice::from_raw_parts(b_data, b_len);
        a_slice == b_slice
    };

    result
}
