/*
 * Silica Runtime System
 *
 * Provides region-based memory management and actor concurrency support
 * for the Silica bootstrap compiler.
 */

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread;
use std::fs;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::ffi::OsStr;

// Basic region structure for memory management
#[repr(C)]
pub struct SilicaRegion {
    pub data: *mut u8,
    pub size: usize,
    pub capacity: usize,
}

// Actor structures for concurrency
#[repr(C)]
pub struct SilicaActor {
    pub id: u64,                    // Unique actor ID
    pub state: i64,                // Actor state
    pub mailbox: *mut VecDeque<i64>, // Per-actor mailbox
    pub behavior_fn: *mut u8,       // Function pointer for behavior (placeholder)
}

// Global actor registry
static mut ACTOR_REGISTRY: Option<std::collections::HashMap<u64, *mut SilicaActor>> = None;
static mut NEXT_ACTOR_ID: u64 = 1;

// Runtime functions that can be called from generated LLVM IR
// Note: LLVM naturally uses C-like calling conventions for external functions
#[no_mangle]
pub extern "C" fn silica_region_create() -> *mut SilicaRegion {
    // Create a new region with initial capacity
    let capacity = 1024; // 1KB initial capacity
    let layout = std::alloc::Layout::from_size_align(capacity, 8).unwrap();
    let data = unsafe { std::alloc::alloc(layout) };

    if data.is_null() {
        panic!("Failed to allocate memory for region");
    }

    let region = Box::new(SilicaRegion {
        data,
        size: 0,
        capacity,
    });

    Box::into_raw(region)
}

#[no_mangle]
pub extern "C" fn silica_region_alloc(region_ptr: *mut SilicaRegion, initial_value: i64) -> *mut i64 {
    if region_ptr.is_null() {
        panic!("Null region pointer");
    }

    let region = unsafe { &mut *region_ptr };

    // Check if we need to grow the region
    let value_size = std::mem::size_of::<i64>();
    if region.size + value_size > region.capacity {
        // For simplicity, we'll just fail if we run out of space
        // A full implementation would grow the region
        panic!("Region out of memory");
    }

    // Allocate space in the region
    let offset = region.size;
    region.size += value_size;

    // Get pointer to the allocated space
    let value_ptr = unsafe { region.data.add(offset) as *mut i64 };

    // Initialize with the provided value
    unsafe { *value_ptr = initial_value };

    value_ptr
}

#[no_mangle]
pub extern "C" fn silica_region_read(ref_ptr: *mut i64) -> i64 {
    if ref_ptr.is_null() {
        panic!("Null reference pointer");
    }

    unsafe { *ref_ptr }
}

#[no_mangle]
pub extern "C" fn silica_region_write(ref_ptr: *mut i64, value: i64) {
    if ref_ptr.is_null() {
        panic!("Null reference pointer");
    }

    unsafe { *ref_ptr = value };
}

#[no_mangle]
pub extern "C" fn silica_region_destroy(region_ptr: *mut SilicaRegion) {
    if region_ptr.is_null() {
        return;
    }

    let region = unsafe { Box::from_raw(region_ptr) };

    // Free the allocated data
    let layout = unsafe {
        std::alloc::Layout::from_size_align_unchecked(region.capacity, 8)
    };
    unsafe { std::alloc::dealloc(region.data, layout) };

    // The region box will be dropped automatically
}

#[no_mangle]
pub extern "C" fn silica_actor_spawn(initial_state: i64, behavior_fn: *mut u8) -> *mut SilicaActor {
    // Initialize actor registry if needed
    unsafe {
        if ACTOR_REGISTRY.is_none() {
            ACTOR_REGISTRY = Some(std::collections::HashMap::new());
        }
    }

    // Generate unique actor ID
    let actor_id = unsafe {
        let id = NEXT_ACTOR_ID;
        NEXT_ACTOR_ID += 1;
        id
    };

    // Create actor mailbox
    let mailbox = Box::new(VecDeque::new());
    let mailbox_ptr = Box::into_raw(mailbox);

    // Create actor structure
    let actor = SilicaActor {
        id: actor_id,
        state: initial_state,
        mailbox: mailbox_ptr,
        behavior_fn,
    };

    let actor_ptr = Box::into_raw(Box::new(actor));

    // Register the actor
    unsafe {
        if let Some(ref mut registry) = ACTOR_REGISTRY {
            registry.insert(actor_id, actor_ptr);
        }
    }

    actor_ptr
}

#[no_mangle]
pub extern "C" fn silica_actor_send(actor_ptr: *mut SilicaActor, message: i64) {
    if actor_ptr.is_null() {
        // Invalid actor pointer
        return;
    }

    unsafe {
        // Get the actor and add message to its mailbox
        let actor = &mut *actor_ptr;
        let mailbox = &mut *actor.mailbox;
        mailbox.push_back(message);
    }
}

#[no_mangle]
pub extern "C" fn silica_actor_recv(actor_ptr: *mut SilicaActor) -> i64 {
    if actor_ptr.is_null() {
        return 0; // Error: invalid actor
    }

    unsafe {
        let actor = &mut *actor_ptr;
        let mailbox = &mut *actor.mailbox;

        // Try to receive a message from this actor's mailbox
        mailbox.pop_front().unwrap_or(0) // Return 0 if no messages
    }
}

// File I/O functions - LLVM-compatible data structures
// Note: Struct layouts follow LLVM/C conventions for IR compatibility
#[repr(C)]
pub struct SilicaString {
    pub data: *mut u8,
    pub length: usize,
}

#[repr(C)]
pub struct SilicaResult {
    pub success: bool,
    pub data: *mut u8,  // Points to SilicaString on success, error message on failure
}

#[repr(C)]
pub struct ProcessResult {
    pub success: bool,
    pub exit_code: i32,
    pub stdout: *mut SilicaString,  // Captured stdout
    pub stderr: *mut SilicaString,  // Captured stderr
}

// LLVM-compatible external function (not C API)
#[no_mangle]
pub extern "C" fn silica_read_file(path: *const u8, path_len: usize) -> SilicaResult {
    // Convert C string to Rust string
    let path_slice = unsafe { std::slice::from_raw_parts(path, path_len) };
    let path_str = match std::str::from_utf8(path_slice) {
        Ok(s) => s,
        Err(_) => return SilicaResult {
            success: false,
            data: create_error_string("Invalid UTF-8 in path"),
        },
    };

    // Read the file
    match fs::read(path_str) {
        Ok(content) => {
            // Create a SilicaString with the file content
            let silica_string = Box::new(SilicaString {
                data: content.as_ptr() as *mut u8,
                length: content.len(),
            });

            // Leak the content to keep it alive (in a real implementation,
            // this would be managed by regions or garbage collection)
            std::mem::forget(content);

            SilicaResult {
                success: true,
                data: Box::into_raw(silica_string) as *mut u8,
            }
        }
        Err(e) => SilicaResult {
            success: false,
            data: create_error_string(&format!("Failed to read file: {}", e)),
        },
    }
}

// LLVM-compatible external function (not C API)
#[no_mangle]
pub extern "C" fn silica_write_file(path: *const u8, path_len: usize, content: *const u8, content_len: usize) -> SilicaResult {
    // Convert path to Rust string
    let path_slice = unsafe { std::slice::from_raw_parts(path, path_len) };
    let path_str = match std::str::from_utf8(path_slice) {
        Ok(s) => s,
        Err(_) => return SilicaResult {
            success: false,
            data: create_error_string("Invalid UTF-8 in path"),
        },
    };

    // Convert content to Rust slice
    let content_slice = unsafe { std::slice::from_raw_parts(content, content_len) };

    // Write the file
    match fs::write(path_str, content_slice) {
        Ok(()) => SilicaResult {
            success: true,
            data: std::ptr::null_mut(), // No data on success
        },
        Err(e) => SilicaResult {
            success: false,
            data: create_error_string(&format!("Failed to write file: {}", e)),
        },
    }
}

fn create_error_string(message: &str) -> *mut u8 {
    let error_string = Box::new(SilicaString {
        data: message.as_ptr() as *mut u8,
        length: message.len(),
    });

    // Leak the message to keep it alive
    std::mem::forget(message.to_owned());

    Box::into_raw(error_string) as *mut u8
}

// Helper function to free SilicaString (LLVM-compatible)
#[no_mangle]
pub extern "C" fn silica_free_string(string_ptr: *mut SilicaString) {
    if !string_ptr.is_null() {
        unsafe {
            let string = Box::from_raw(string_ptr);
            // In a real implementation, we would need to track how the data was allocated
            // For now, we'll assume the data was leaked and we can't free it safely
            drop(string);
        }
    }
}

// Process execution functions
#[no_mangle]
pub extern "C" fn silica_exec_command(
    cmd: *const u8,
    cmd_len: usize,
    args_ptr: *const *const u8,  // Array of string pointers
    args_len: usize,             // Number of arguments
    arg_lengths: *const usize,   // Array of string lengths
) -> *mut ProcessResult {
    // Convert command to Rust string
    let cmd_slice = unsafe { std::slice::from_raw_parts(cmd, cmd_len) };
    let command = match std::str::from_utf8(cmd_slice) {
        Ok(s) => s,
        Err(_) => return create_error_process_result("Invalid UTF-8 in command"),
    };

    // Convert arguments to Rust strings
    let mut rust_args = Vec::new();
    for i in 0..args_len {
        let arg_ptr = unsafe { *args_ptr.add(i) };
        let arg_len = unsafe { *arg_lengths.add(i) };
        let arg_slice = unsafe { std::slice::from_raw_parts(arg_ptr, arg_len) };
        let arg_str = match std::str::from_utf8(arg_slice) {
            Ok(s) => s,
            Err(_) => return create_error_process_result("Invalid UTF-8 in argument"),
        };
        rust_args.push(arg_str.to_string());
    }

    // Execute the command
    let mut cmd_builder = Command::new(command);
    for arg in rust_args {
        cmd_builder.arg(arg);
    }

    // Capture stdout and stderr
    cmd_builder.stdout(Stdio::piped());
    cmd_builder.stderr(Stdio::piped());

    match cmd_builder.output() {
        Ok(output) => {
            // Create SilicaStrings for stdout and stderr
            let stdout_string = create_silica_string(&output.stdout);
            let stderr_string = create_silica_string(&output.stderr);

            let result = ProcessResult {
                success: output.status.success(),
                exit_code: output.status.code().unwrap_or(-1),
                stdout: Box::into_raw(Box::new(stdout_string)),
                stderr: Box::into_raw(Box::new(stderr_string)),
            };

            Box::into_raw(Box::new(result))
        }
        Err(e) => create_error_process_result(&format!("Failed to execute command: {}", e)),
    }
}

// Helper functions
fn create_silica_string(data: &[u8]) -> SilicaString {
    let silica_string = SilicaString {
        data: data.as_ptr() as *mut u8,
        length: data.len(),
    };

    // Leak the data to keep it alive
    std::mem::forget(data.to_owned());

    silica_string
}

fn create_error_process_result(message: &str) -> *mut ProcessResult {
    let error_string = SilicaString {
        data: message.as_ptr() as *mut u8,
        length: message.len(),
    };

    // Leak the message
    std::mem::forget(message.to_owned());

    let result = ProcessResult {
        success: false,
        exit_code: -1,
        stdout: std::ptr::null_mut(),
        stderr: Box::into_raw(Box::new(error_string)),
    };

    Box::into_raw(Box::new(result))
}

// Helper function to free ProcessResult
#[no_mangle]
pub extern "C" fn silica_free_process_result(result_ptr: *mut ProcessResult) {
    if !result_ptr.is_null() {
        unsafe {
            let result = Box::from_raw(result_ptr);
            if !result.stdout.is_null() {
                silica_free_string(result.stdout);
            }
            if !result.stderr.is_null() {
                silica_free_string(result.stderr);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_operations() {
        // Create a region
        let region = silica_region_create();
        assert!(!region.is_null());

        // Allocate a value
        let ref_ptr = silica_region_alloc(region, 42);
        assert!(!ref_ptr.is_null());

        // Read the value
        let value = silica_region_read(ref_ptr);
        assert_eq!(value, 42);

        // Write a new value
        silica_region_write(ref_ptr, 24);
        let new_value = silica_region_read(ref_ptr);
        assert_eq!(new_value, 24);

        // Clean up
        silica_region_destroy(region);
    }

    #[test]
    fn test_actor_operations() {
        // Spawn an actor
        let actor_ptr = silica_actor_spawn(100, std::ptr::null_mut());
        assert!(!actor_ptr.is_null());

        // Send a message
        silica_actor_send(actor_ptr, 42);

        // Receive a message
        let message = silica_actor_recv();
        assert_eq!(message, 42);

        // Clean up
        unsafe {
            let _ = Box::from_raw(actor_ptr);
        }
    }
}
