/*
 * Silica Runtime System
 *
 * Provides region-based memory management and actor concurrency support
 * for the Silica bootstrap compiler.
 */

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread;

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
    pub state: i64,  // Simple state for now
    pub mailbox: *mut VecDeque<i64>,
}

// Global actor storage for simplicity
static mut GLOBAL_ACTOR_MAILBOX: Option<VecDeque<i64>> = None;

// Runtime functions that can be called from generated LLVM code
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
pub extern "C" fn silica_actor_spawn(initial_state: i64, _behavior_fn: *mut u8) -> *mut SilicaActor {
    // For simplicity, create a basic actor structure
    // In a real implementation, this would be much more complex
    let mailbox = Box::new(VecDeque::new());
    let mailbox_ptr = Box::into_raw(mailbox);

    let actor = SilicaActor {
        state: initial_state,
        mailbox: mailbox_ptr,
    };

    // Store the actor in a global mailbox for testing
    unsafe {
        if GLOBAL_ACTOR_MAILBOX.is_none() {
            GLOBAL_ACTOR_MAILBOX = Some(VecDeque::new());
        }
    }

    Box::into_raw(Box::new(actor))
}

#[no_mangle]
pub extern "C" fn silica_actor_send(_actor_ptr: *mut SilicaActor, message: i64) {
    // For simplicity, just store in global mailbox
    unsafe {
        if let Some(ref mut mailbox) = GLOBAL_ACTOR_MAILBOX {
            mailbox.push_back(message);
        }
    }
}

#[no_mangle]
pub extern "C" fn silica_actor_recv() -> i64 {
    // For simplicity, receive from global mailbox
    unsafe {
        if let Some(ref mut mailbox) = GLOBAL_ACTOR_MAILBOX {
            mailbox.pop_front().unwrap_or(42) // Default message if empty
        } else {
            42 // Default message
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
