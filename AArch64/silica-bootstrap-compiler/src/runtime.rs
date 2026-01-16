/*
 * Silica Runtime System
 *
 * Provides region-based memory management and actor concurrency support
 * for the Silica bootstrap compiler.
 */

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::fs;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::ffi::OsStr;
use std::mem;

// CPU Affinity support for macOS/AArch64
// TODO: Implement actual macOS CPU affinity using pthreads API

// CPU Topology Detection for macOS/AArch64
#[cfg(target_os = "macos")]
mod topology {
    use std::mem;
    use std::ptr;

    // macOS sysctl constants for CPU topology
    const CTL_HW: libc::c_int = 6;
    const HW_NCPU: libc::c_int = 3;
    const HW_AVAILCPU: libc::c_int = 25;

    // CPU core type information
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum CoreType {
        Performance,
        Efficiency,
        Unknown,
    }

    #[derive(Debug, Clone)]
    pub struct CpuCore {
        pub id: i32,
        pub core_type: CoreType,
        pub capacity: Option<u32>,
    }

    // Safe wrapper for sysctl
    fn sysctl(name: &[libc::c_int], oldp: *mut libc::c_void, oldlenp: *mut libc::size_t) -> libc::c_int {
        unsafe { libc::sysctl(name.as_ptr() as *mut _, name.len() as u32, oldp, oldlenp, ptr::null_mut(), 0) }
    }

    // Get number of CPUs using sysctl
    pub fn get_cpu_count() -> Result<i32, String> {
        let mut count: libc::c_int = 0;
        let mut size = mem::size_of::<libc::c_int>();
        let name = [CTL_HW, HW_NCPU];

        let result = sysctl(&name, &mut count as *mut _ as *mut libc::c_void, &mut size);
        if result == 0 {
            Ok(count)
        } else {
            Err(format!("sysctl failed to get CPU count: {}", result))
        }
    }

    // Detect core types using sysctlbyname
    // On macOS, we can use various sysctl MIBs to detect CPU topology
    pub fn detect_core_types() -> Result<Vec<CpuCore>, String> {
        let cpu_count = get_cpu_count()?;

        // Try to detect if this is an Apple Silicon system with different core types
        // This is a simplified approach - in practice, Apple Silicon systems have
        // performance cores (Firestorm) and efficiency cores (Icestorm)

        let mut cores = Vec::new();

        // For Apple Silicon, we typically have:
        // - Performance cores: lower-numbered cores (usually cores 0-3)
        // - Efficiency cores: higher-numbered cores (usually cores 4+)

        // Use heuristics based on CPU count and sysctl information
        // In a full implementation, this would query more detailed CPUID-like information

        for i in 0..cpu_count {
            let core_type = if cpu_count <= 4 {
                // Single cluster systems - all cores are performance
                CoreType::Performance
            } else if i < cpu_count / 2 {
                // First half are typically performance cores
                CoreType::Performance
            } else {
                // Second half are typically efficiency cores
                CoreType::Efficiency
            };

            cores.push(CpuCore {
                id: i,
                core_type,
                capacity: None, // macOS sysctl doesn't provide capacity info
            });
        }

        // Try to get more accurate information using sysctlbyname
        // This is a more advanced approach that could be implemented
        enhance_with_sysctlbyname(&mut cores)?;

        Ok(cores)
    }

    // Try to enhance core type detection with sysctlbyname
    fn enhance_with_sysctlbyname(cores: &mut Vec<CpuCore>) -> Result<(), String> {
        // This would use sysctlbyname to get more detailed CPU information
        // For now, we keep the basic heuristics
        // In a full implementation, this could query:
        // - "hw.cpusubtype" for CPU subtype information
        // - "hw.cpufamily" for CPU family
        // - "machdep.cpu.brand_string" for CPU brand
        // - And potentially use IOKit or other APIs for more detailed topology

        // For Apple Silicon detection, we could look for:
        // - CPU brand strings containing "Apple" or specific model names
        // - Cache topology information
        // - Power management capabilities

        Ok(())
    }

    // Get performance cores (cores that can run at high frequency)
    pub fn get_performance_cores() -> Vec<i32> {
        detect_core_types()
            .unwrap_or_else(|_| {
                // Fallback to heuristics if detection fails - create CpuCore vector
                let cpu_count = get_cpu_count().unwrap_or(4);
                let mut cores = Vec::new();
                for i in 0..cpu_count {
                    let core_type = if cpu_count <= 4 {
                        CoreType::Performance
                    } else if i < cpu_count / 2 {
                        CoreType::Performance
                    } else {
                        CoreType::Efficiency
                    };
                    cores.push(CpuCore { id: i, core_type, capacity: None });
                }
                cores
            })
            .into_iter()
            .filter(|core| core.core_type == CoreType::Performance)
            .map(|core| core.id)
            .collect()
    }

    // Get efficiency cores (cores optimized for low power)
    pub fn get_efficiency_cores() -> Vec<i32> {
        detect_core_types()
            .unwrap_or_else(|_| {
                // Fallback to heuristics if detection fails - create CpuCore vector
                let cpu_count = get_cpu_count().unwrap_or(4);
                let mut cores = Vec::new();
                for i in 0..cpu_count {
                    let core_type = if cpu_count <= 4 {
                        CoreType::Performance
                    } else if i < cpu_count / 2 {
                        CoreType::Performance
                    } else {
                        CoreType::Efficiency
                    };
                    cores.push(CpuCore { id: i, core_type, capacity: None });
                }
                cores
            })
            .into_iter()
            .filter(|core| core.core_type == CoreType::Efficiency)
            .map(|core| core.id)
            .collect()
    }
}

// Linux/AArch64 CPU topology detection using sysfs
#[cfg(all(target_arch = "aarch64", target_os = "linux"))]
mod topology {
    use std::fs;
    use std::path::Path;
    use std::io::Read;

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum CoreType {
        Performance,
        Efficiency,
        Unknown,
    }

    #[derive(Debug, Clone)]
    pub struct CpuCore {
        pub id: i32,
        pub core_type: CoreType,
        pub capacity: Option<u32>, // CPU capacity (relative performance)
    }

    // Read CPU capacity from sysfs
    fn read_cpu_capacity(cpu_id: i32) -> Option<u32> {
        let path = format!("/sys/devices/system/cpu/cpu{}/cpu_capacity", cpu_id);
        if let Ok(mut file) = fs::File::open(&path) {
            let mut contents = String::new();
            if file.read_to_string(&mut contents).is_ok() {
                contents.trim().parse::<u32>().ok()
            } else {
                None
            }
        } else {
            None
        }
    }

    // Check if CPU core exists
    fn cpu_exists(cpu_id: i32) -> bool {
        Path::new(&format!("/sys/devices/system/cpu/cpu{}", cpu_id)).exists()
    }

    // Get maximum CPU capacity across all cores
    fn get_max_capacity(cores: &[CpuCore]) -> u32 {
        cores.iter()
            .filter_map(|core| core.capacity)
            .max()
            .unwrap_or(1024) // Default to 1024 if no capacity info
    }

    // Detect core types based on capacity information
    pub fn detect_core_types() -> Result<Vec<CpuCore>, String> {
        let mut cores = Vec::new();
        let mut cpu_id = 0;

        // Enumerate all CPU cores
        while cpu_exists(cpu_id) {
            let capacity = read_cpu_capacity(cpu_id);
            cores.push(CpuCore {
                id: cpu_id,
                core_type: CoreType::Unknown, // Will be determined below
                capacity,
            });
            cpu_id += 1;
        }

        if cores.is_empty() {
            return Err("No CPU cores detected".to_string());
        }

        // Classify cores based on capacity relative to maximum
        let max_capacity = get_max_capacity(&cores);
        let capacity_threshold = (max_capacity as f32 * 0.8) as u32; // 80% of max is efficiency

        for core in &mut cores {
            core.core_type = match core.capacity {
                Some(cap) if cap >= max_capacity => CoreType::Performance,
                Some(cap) if cap >= capacity_threshold => CoreType::Performance,
                Some(_) => CoreType::Efficiency,
                None => {
                    // No capacity info - fall back to position-based heuristics
                    if core.id < (cores.len() / 2) as i32 {
                        CoreType::Performance
                    } else {
                        CoreType::Efficiency
                    }
                }
            };
        }

        Ok(cores)
    }

    // Get CPU count using sysfs
    pub fn get_cpu_count() -> Result<i32, String> {
        let mut count = 0;
        while cpu_exists(count) {
            count += 1;
        }

        if count == 0 {
            Err("No CPUs detected".to_string())
        } else {
            Ok(count)
        }
    }

    pub fn get_performance_cores() -> Vec<i32> {
        detect_core_types()
            .unwrap_or_else(|_| vec![])
            .into_iter()
            .filter(|core| core.core_type == CoreType::Performance)
            .map(|core| core.id)
            .collect()
    }

    pub fn get_efficiency_cores() -> Vec<i32> {
        detect_core_types()
            .unwrap_or_else(|_| vec![])
            .into_iter()
            .filter(|core| core.core_type == CoreType::Efficiency)
            .map(|core| core.id)
            .collect()
    }
}

// Android/AArch64 - similar to Linux but with Android-specific features
#[cfg(all(target_arch = "aarch64", target_os = "android"))]
mod topology {
    // For Android, we can use the Linux implementation as a base
    // but could add Android-specific thermal management integration
    use std::fs;
    use std::io::Read;

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum CoreType {
        Performance,
        Efficiency,
        Unknown,
    }

    #[derive(Debug, Clone)]
    pub struct CpuCore {
        pub id: i32,
        pub core_type: CoreType,
        pub capacity: Option<u32>,
    }

    fn read_cpu_capacity(cpu_id: i32) -> Option<u32> {
        let path = format!("/sys/devices/system/cpu/cpu{}/cpu_capacity", cpu_id);
        if let Ok(mut file) = fs::File::open(&path) {
            let mut contents = String::new();
            if file.read_to_string(&mut contents).is_ok() {
                contents.trim().parse::<u32>().ok()
            } else {
                None
            }
        } else {
            None
        }
    }

    fn cpu_exists(cpu_id: i32) -> bool {
        std::path::Path::new(&format!("/sys/devices/system/cpu/cpu{}", cpu_id)).exists()
    }

    fn get_max_capacity(cores: &[CpuCore]) -> u32 {
        cores.iter()
            .filter_map(|core| core.capacity)
            .max()
            .unwrap_or(1024)
    }

    pub fn detect_core_types() -> Result<Vec<CpuCore>, String> {
        let mut cores = Vec::new();
        let mut cpu_id = 0;

        while cpu_exists(cpu_id) {
            let capacity = read_cpu_capacity(cpu_id);
            cores.push(CpuCore {
                id: cpu_id,
                core_type: CoreType::Unknown,
                capacity,
            });
            cpu_id += 1;
        }

        if cores.is_empty() {
            return Err("No CPU cores detected".to_string());
        }

        let max_capacity = get_max_capacity(&cores);
        let capacity_threshold = (max_capacity as f32 * 0.8) as u32;

        for core in &mut cores {
            core.core_type = match core.capacity {
                Some(cap) if cap >= capacity_threshold => CoreType::Performance,
                Some(_) => CoreType::Efficiency,
                None => {
                    // Android fallback - often cores 4+ are efficiency cores
                    if core.id >= 4 {
                        CoreType::Efficiency
                    } else {
                        CoreType::Performance
                    }
                }
            };
        }

        Ok(cores)
    }

    pub fn get_cpu_count() -> Result<i32, String> {
        let mut count = 0;
        while cpu_exists(count) {
            count += 1;
        }

        if count == 0 {
            Err("No CPUs detected".to_string())
        } else {
            Ok(count)
        }
    }

    pub fn get_performance_cores() -> Vec<i32> {
        detect_core_types()
            .unwrap_or_else(|_| vec![])
            .into_iter()
            .filter(|core| core.core_type == CoreType::Performance)
            .map(|core| core.id)
            .collect()
    }

    pub fn get_efficiency_cores() -> Vec<i32> {
        detect_core_types()
            .unwrap_or_else(|_| vec![])
            .into_iter()
            .filter(|core| core.core_type == CoreType::Efficiency)
            .map(|core| core.id)
            .collect()
    }
}

// Fallback implementation for non-macOS, non-Linux-AArch64 platforms
#[cfg(not(any(
    target_os = "macos",
    all(target_arch = "aarch64", target_os = "linux"),
    all(target_arch = "aarch64", target_os = "android")
)))]
mod topology {
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum CoreType {
        Performance,
        Efficiency,
        Unknown,
    }

    #[derive(Debug, Clone)]
    pub struct CpuCore {
        pub id: i32,
        pub core_type: CoreType,
        pub capacity: Option<u32>,
    }

    pub fn detect_core_types() -> Result<Vec<CpuCore>, String> {
        // Fallback: use simple heuristics
        let cpu_count = crate::get_available_cores();
        let mut cores = Vec::new();

        for i in 0..cpu_count {
            let core_type = if cpu_count <= 4 {
                CoreType::Performance
            } else if i < cpu_count / 2 {
                CoreType::Performance
            } else {
                CoreType::Efficiency
            };

            cores.push(CpuCore {
                id: i,
                core_type,
                capacity: None, // No capacity info in fallback
            });
        }

        Ok(cores)
    }

    pub fn get_performance_cores() -> Vec<i32> {
        detect_core_types()
            .unwrap_or_else(|_| vec![])
            .into_iter()
            .filter(|core| core.core_type == CoreType::Performance)
            .map(|core| core.id)
            .collect()
    }

    pub fn get_efficiency_cores() -> Vec<i32> {
        detect_core_types()
            .unwrap_or_else(|_| vec![])
            .into_iter()
            .filter(|core| core.core_type == CoreType::Efficiency)
            .map(|core| core.id)
            .collect()
    }
}

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
    pub state: *mut u8,             // Actor state (generic pointer for any type)
    pub mailbox: Arc<Mutex<VecDeque<*mut u8>>>, // Synchronized mailbox
    pub behavior_fn: *mut u8,       // Function pointer for behavior
}

// Safety: SilicaActor is Send because:
// - The actor is created in the main thread
// - The state pointer is allocated once and only modified by the actor's own thread
// - The mailbox is properly synchronized with Arc<Mutex<>>
// - The behavior function pointer is read-only
unsafe impl Send for SilicaActor {}


// Global actor registry
static mut ACTOR_REGISTRY: Option<std::collections::HashMap<u64, Arc<Mutex<SilicaActor>>>> = None;
static mut NEXT_ACTOR_ID: u64 = 1;

// Store actor pointers for C API - maps raw pointers back to Arc references
// Using OnceLock for thread-safe initialization and access
// Using usize as key (pointer value) instead of *mut to satisfy Send requirement
static ACTOR_PTR_MAP: OnceLock<Mutex<std::collections::HashMap<usize, Arc<Mutex<SilicaActor>>>>> = OnceLock::new();

// Core affinity load balancing
static mut NEXT_CORE_ID: i32 = 0;
static mut NEXT_PERFORMANCE_CORE: i32 = 0;
static mut NEXT_EFFICIENCY_CORE: i32 = 0;


// Helper function to start an actor's message processing loop
fn start_actor_message_loop(actor: Arc<Mutex<SilicaActor>>, core_affinity: i32) {
    let actor_id = {
        let actor_guard = actor.lock().unwrap();
        actor_guard.id
    };
    // eprintln!("[DEBUG] start_actor_message_loop: Starting message loop for actor ID {} with core_affinity={}", actor_id, core_affinity);
    // Spawn a new thread for the actor's message loop
    std::thread::spawn(move || {
        // eprintln!("[DEBUG] start_actor_message_loop: Thread spawned for actor ID {}", actor_id);
        // Set CPU affinity if specified (core_affinity != 0)
        if core_affinity != 0 {
            set_thread_affinity(core_affinity as u32);
        }
        actor_message_loop(actor);
    });
}

/// Set CPU affinity for the current thread (macOS/AArch64 implementation)
#[cfg(target_os = "macos")]
fn set_thread_affinity(core_id: u32) {
    // TODO: Implement macOS CPU affinity using pthreads API
    // For now, this is a no-op to allow compilation
    // In a full implementation, this would use:
    // - pthread_self() to get current thread
    // - pthread_setaffinity_np() to set CPU affinity
    // - CPU_SET macros to manipulate cpu_set_t
    let _ = core_id; // Suppress unused parameter warning
}

/// Get the number of available CPU cores
fn get_available_cores() -> i32 {
    // Try topology detection first (macOS specific)
    #[cfg(target_os = "macos")]
    {
        if let Ok(count) = topology::get_cpu_count() {
            return count;
        }
    }

    // Fallback: use num_cpus crate if available
    #[cfg(feature = "num_cpus")]
    {
        num_cpus::get() as i32
    }

    // Final fallback: environment variable or default
    #[cfg(not(feature = "num_cpus"))]
    {
        std::env::var("SILICA_CPU_CORES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(4)
    }
}

/// Get detailed CPU topology information for debugging
pub fn get_cpu_topology_info() -> String {
    match topology::detect_core_types() {
        Ok(cores) => {
            let perf_cores: Vec<_> = cores.iter().filter(|c| c.core_type == topology::CoreType::Performance).collect();
            let eff_cores: Vec<_> = cores.iter().filter(|c| c.core_type == topology::CoreType::Efficiency).collect();

            let capacity_info = cores.iter()
                .filter_map(|c| c.capacity.map(|cap| format!("cpu{}:{}", c.id, cap)))
                .collect::<Vec<_>>()
                .join(", ");

            format!(
                "CPU Topology: {} total cores, {} performance cores ({:?}), {} efficiency cores ({:?}) [capacities: {}]",
                cores.len(),
                perf_cores.len(),
                perf_cores.iter().map(|c| c.id).collect::<Vec<_>>(),
                eff_cores.len(),
                eff_cores.iter().map(|c| c.id).collect::<Vec<_>>(),
                if capacity_info.is_empty() { "N/A".to_string() } else { capacity_info }
            )
        }
        Err(e) => format!("CPU Topology detection failed: {}", e),
    }
}

/// Select the next available core using round-robin load balancing
fn select_next_core() -> i32 {
    unsafe {
        let core_count = get_available_cores();
        let selected_core = NEXT_CORE_ID;
        NEXT_CORE_ID = (NEXT_CORE_ID + 1) % core_count;
        selected_core
    }
}

/// Get performance cores using runtime topology detection
fn get_performance_cores() -> Vec<i32> {
    topology::get_performance_cores()
}

/// Get efficiency cores using runtime topology detection
fn get_efficiency_cores() -> Vec<i32> {
    topology::get_efficiency_cores()
}

/// Select next performance core using round-robin
fn select_next_performance_core() -> i32 {
    let perf_cores = get_performance_cores();
    if perf_cores.is_empty() {
        return select_next_core(); // Fallback to any core
    }

    unsafe {
        let core_count = perf_cores.len() as i32;
        let selected_idx = NEXT_PERFORMANCE_CORE % core_count;
        let selected_core = perf_cores[selected_idx as usize];
        NEXT_PERFORMANCE_CORE = (NEXT_PERFORMANCE_CORE + 1) % core_count;
        selected_core
    }
}

/// Select next efficiency core using round-robin
fn select_next_efficiency_core() -> i32 {
    let eff_cores = get_efficiency_cores();
    if eff_cores.is_empty() {
        return select_next_core(); // Fallback to any core
    }

    unsafe {
        let core_count = eff_cores.len() as i32;
        let selected_idx = NEXT_EFFICIENCY_CORE % core_count;
        let selected_core = eff_cores[selected_idx as usize];
        NEXT_EFFICIENCY_CORE = (NEXT_EFFICIENCY_CORE + 1) % core_count;
        selected_core
    }
}

/// Set CPU affinity for the current thread (fallback for other platforms)
#[cfg(not(target_os = "macos"))]
fn set_thread_affinity(_core_id: u32) {
    // CPU affinity not implemented for this platform
    // This is a no-op that allows compilation
}


// Actor message processing loop
// Continuously receives messages and processes them using the behavior function
fn actor_message_loop(actor: Arc<Mutex<SilicaActor>>) {
    let actor_id = {
        let actor_guard = actor.lock().unwrap();
        actor_guard.id
    };
    // eprintln!("[DEBUG] actor_message_loop: Started for actor ID {}", actor_id);
    let mut loop_count = 0;
    loop {
        loop_count += 1;
        if loop_count % 100 == 0 {
            // eprintln!("[DEBUG] actor_message_loop: Loop iteration {} for actor {}", loop_count, actor_id);
        }
        
        // Try to receive a message (non-blocking for now)
        let (message_ptr, mailbox_size) = {
            let mut actor_guard = actor.lock().unwrap();
            let mut mailbox = actor_guard.mailbox.lock().unwrap();
            let size = mailbox.len();
            let msg = mailbox.pop_front().unwrap_or(std::ptr::null_mut());
            (msg, size)
        };

        if !message_ptr.is_null() {
            // eprintln!("[DEBUG] actor_message_loop: Received message {:p}, mailbox had {} messages", message_ptr, mailbox_size);
            // Get the current actor state and behavior function
            let (state_ptr, behavior_fn_ptr) = {
                let actor_guard = actor.lock().unwrap();
                (actor_guard.state, actor_guard.behavior_fn)
            };

            // Call the behavior function if available
            if !behavior_fn_ptr.is_null() {
                // eprintln!("[DEBUG] actor_message_loop: Calling behavior function");
                // Cast the behavior function pointer to the uniform interface
                // Behavior functions: fn(*mut u8 message, *mut u8 state) -> *mut u8 new_state
                let behavior_fn = unsafe {
                    std::mem::transmute::<*mut u8, unsafe extern "C" fn(*mut u8, *mut u8) -> *mut u8>(behavior_fn_ptr)
                };

                // Call behavior function: (message_ptr, state_ptr) -> new_state_ptr
                let new_state_ptr = unsafe { behavior_fn(message_ptr, state_ptr) };
                // eprintln!("[DEBUG] actor_message_loop: Behavior function returned {:p}", new_state_ptr);

                // Update the actor's state pointer
                // Note: For the bootstrap compiler, behavior functions typically return the same pointer
                // or a modified version. For now, we'll update the state pointer.
                if !new_state_ptr.is_null() && new_state_ptr != state_ptr {
                    // If the function returned a different pointer, update the actor's state
                    let mut actor_guard = actor.lock().unwrap();
                    actor_guard.state = new_state_ptr;
                }
            } else {
                // eprintln!("[DEBUG] actor_message_loop: Behavior function pointer is null!");
            }

            // Free the message memory after processing
            // Note: In the bootstrap runtime, we don't actually free memory
            // as the process will exit. In a full runtime, this would be important.
        } else if mailbox_size > 0 {
            // eprintln!("[DEBUG] actor_message_loop: No message popped but mailbox size was {}", mailbox_size);
        }

        // Small delay to avoid busy waiting
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

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
pub extern "C" fn silica_actor_spawn(initial_state: *mut u8, behavior_fn: *mut u8, core_affinity: i32) -> *mut SilicaActor {
    // eprintln!("[DEBUG] silica_actor_spawn: Called with initial_state={:p}, behavior_fn={:p}, core_affinity={}", 
    //           initial_state, behavior_fn, core_affinity);
    // Determine actual core affinity based on the requested type
    let actual_core_affinity = match core_affinity {
        0 => select_next_core(), // Any core - load balanced
        -1 => select_next_performance_core(), // Performance cores - load balanced within group
        -2 => select_next_efficiency_core(), // Efficiency cores - load balanced within group
        positive if positive > 0 => positive, // Specific core ID
        _ => select_next_core(), // Unknown negative values - fallback to any core
    };
    // eprintln!("[DEBUG] silica_actor_spawn: Actual core affinity={}", actual_core_affinity);

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
    // eprintln!("[DEBUG] silica_actor_spawn: Generated actor ID {}", actor_id);

    // Create synchronized actor mailbox
    let mailbox = Arc::new(Mutex::new(VecDeque::new()));

    // Create actor structure
    let actor = Arc::new(Mutex::new(SilicaActor {
        id: actor_id,
        state: initial_state,
        mailbox: mailbox.clone(),
        behavior_fn,
    }));
    // eprintln!("[DEBUG] silica_actor_spawn: Created actor with ID {}", actor_id);

    // Get a raw pointer for the C API
    let actor_ptr = Arc::as_ptr(&actor) as *mut SilicaActor;
    // eprintln!("[DEBUG] silica_actor_spawn: Actor pointer is {:p} (usize: {})", actor_ptr, actor_ptr as usize);

    // Register the actor and store pointer mapping
    unsafe {
        if let Some(ref mut registry) = ACTOR_REGISTRY {
            registry.insert(actor_id, actor.clone());
            // eprintln!("[DEBUG] silica_actor_spawn: Registered actor {} in ACTOR_REGISTRY", actor_id);
        }
        // Thread-safe initialization and access using OnceLock
        // Cast pointer to usize for use as HashMap key (usize is Send)
        let ptr_map = ACTOR_PTR_MAP.get_or_init(|| Mutex::new(std::collections::HashMap::new()));
        ptr_map.lock().unwrap().insert(actor_ptr as usize, actor.clone());
        // eprintln!("[DEBUG] silica_actor_spawn: Registered actor {} in ACTOR_PTR_MAP with key {}", actor_id, actor_ptr as usize);
    }

    // Start the actor's message processing loop in a new thread
    // eprintln!("[DEBUG] silica_actor_spawn: Starting message loop for actor {}", actor_id);
    start_actor_message_loop(actor, actual_core_affinity);

    // Return the raw pointer for the C API
    // eprintln!("[DEBUG] silica_actor_spawn: Returning actor pointer {:p}", actor_ptr);
    actor_ptr
}

#[no_mangle]
pub extern "C" fn silica_actor_send(actor_ptr: *mut SilicaActor, message: *mut u8) {
    if actor_ptr.is_null() {
        // Invalid actor pointer
        return;
    }

    // Get the Arc back from the pointer (thread-safe access)
    // Cast pointer to usize for HashMap lookup
    if let Some(ptr_map) = ACTOR_PTR_MAP.get() {
        let map_guard = ptr_map.lock().unwrap();
        if let Some(actor_arc) = map_guard.get(&(actor_ptr as usize)) {
            let actor_guard = actor_arc.lock().unwrap();
            let mut mailbox = actor_guard.mailbox.lock().unwrap();
            mailbox.push_back(message);
            // Note: No condvar notification for now - receiver will poll
        }
    }
}

#[no_mangle]
pub extern "C" fn silica_actor_recv(actor_ptr: *mut SilicaActor) -> *mut u8 {
    if actor_ptr.is_null() {
        return std::ptr::null_mut(); // Error: invalid actor
    }

    // Get the Arc back from the pointer (thread-safe access)
    // Cast pointer to usize for HashMap lookup
    if let Some(ptr_map) = ACTOR_PTR_MAP.get() {
        let map_guard = ptr_map.lock().unwrap();
        if let Some(actor_arc) = map_guard.get(&(actor_ptr as usize)) {
            let actor_guard = actor_arc.lock().unwrap();
            let mut mailbox = actor_guard.mailbox.lock().unwrap();
            return mailbox.pop_front().unwrap_or(std::ptr::null_mut());
        }
    }

    std::ptr::null_mut() // Error or no messages
}

#[no_mangle]
pub extern "C" fn silica_actor_cast(actor: *mut SilicaActor, message: *mut u8) -> bool {
    // eprintln!("[DEBUG] silica_actor_cast: Called with actor={:p}, message={:p}", actor, message);
    // Non-blocking: enqueue message and return immediately
    // Returns true if message successfully enqueued, false if actor doesn't exist or mailbox full
    if actor.is_null() {
        // eprintln!("[DEBUG] silica_actor_cast: Actor is null, returning false");
        return false;
    }

    // Get the Arc back from the pointer (thread-safe access)
    // Cast pointer to usize for HashMap lookup
    if let Some(ptr_map) = ACTOR_PTR_MAP.get() {
        let map_guard = ptr_map.lock().unwrap();
        if let Some(actor_arc) = map_guard.get(&(actor as usize)) {
            let actor_guard = actor_arc.lock().unwrap();
            let mut mailbox = actor_guard.mailbox.lock().unwrap();
            let mailbox_size_before = mailbox.len();
            mailbox.push_back(message);
            let mailbox_size_after = mailbox.len();
            // eprintln!("[DEBUG] silica_actor_cast: Enqueued message, mailbox size: {} -> {}", mailbox_size_before, mailbox_size_after);
            return true;
        } else {
            // eprintln!("[DEBUG] silica_actor_cast: Actor not found in map for pointer {:p} (usize: {})", actor, actor as usize);
        }
    } else {
        // eprintln!("[DEBUG] silica_actor_cast: ACTOR_PTR_MAP not initialized!");
    }
    // eprintln!("[DEBUG] silica_actor_cast: Returning false");
    false
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

    // Write/append to the file (append_file intrinsic uses this)
    match fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path_str) {
        Ok(mut file) => {
            use std::io::Write;
            match file.write_all(content_slice) {
        Ok(()) => SilicaResult {
            success: true,
            data: std::ptr::null_mut(), // No data on success
        },
        Err(e) => SilicaResult {
            success: false,
                    data: create_error_string(&format!("Failed to write to file: {}", e)),
                },
            }
        }
        Err(e) => SilicaResult {
            success: false,
            data: create_error_string(&format!("Failed to open file: {}", e)),
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

#[no_mangle]
pub extern "C" fn silica_get_cpu_topology_info() -> *mut u8 {
    let info_string = get_cpu_topology_info();

    // Create a SilicaString with the topology info
    let content_bytes = info_string.as_bytes();
    let silica_string = Box::new(SilicaString {
        data: content_bytes.as_ptr() as *mut u8,
        length: content_bytes.len(),
    });

    // Return the boxed SilicaString as a raw pointer
    Box::into_raw(silica_string) as *mut u8
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
        let message = silica_actor_recv(actor_ptr);
        assert_eq!(message, 42);

        // Clean up
        unsafe {
            let _ = Box::from_raw(actor_ptr);
        }
    }
}
