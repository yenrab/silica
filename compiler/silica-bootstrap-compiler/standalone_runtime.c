#include <stdlib.h>
#include <stdio.h>
#include <string.h>

// Silica standalone runtime - C implementations of runtime functions
// This allows LLVM-generated code to run as standalone executables

// Silica types
typedef struct SilicaRegion {
    void* data;
    size_t size;
    size_t capacity;
} SilicaRegion;

typedef struct SilicaString {
    char* data;
    size_t len;
} SilicaString;

typedef struct SilicaResult {
    int success;
    void* data;
} SilicaResult;

typedef struct SilicaActor {
    unsigned long long id;
    long long state;
    void* mailbox; // Actually a VecDeque, but we use void* for simplicity
    void* behavior_fn;
} SilicaActor;

typedef struct ProcessResult {
    int success;
    int exit_code;
    SilicaString stdout;
    SilicaString stderr;
} ProcessResult;

// Memory management functions (region-based)
void* silica_region_create() {
    SilicaRegion* region = (SilicaRegion*)malloc(sizeof(SilicaRegion));
    if (!region) return NULL;

    region->capacity = 1024; // 1KB initial capacity
    region->size = 0;
    region->data = malloc(region->capacity);

    if (!region->data) {
        free(region);
        return NULL;
    }

    return region;
}

void* silica_region_alloc(void* region_ptr, long long initial_value) {
    if (!region_ptr) return NULL;

    SilicaRegion* region = (SilicaRegion*)region_ptr;

    size_t value_size = sizeof(long long);
    if (region->size + value_size > region->capacity) {
        // Simple growth strategy
        size_t new_capacity = region->capacity * 2;
        void* new_data = realloc(region->data, new_capacity);
        if (!new_data) return NULL;

        region->data = new_data;
        region->capacity = new_capacity;
    }

    long long* value_ptr = (long long*)((char*)region->data + region->size);
    *value_ptr = initial_value;
    region->size += value_size;

    return value_ptr;
}

long long silica_region_read(void* ref_ptr) {
    if (!ref_ptr) return 0;
    return *(long long*)ref_ptr;
}

void silica_region_write(void* ref_ptr, long long value) {
    if (ref_ptr) {
        *(long long*)ref_ptr = value;
    }
}

void silica_region_destroy(void* region_ptr) {
    if (region_ptr) {
        SilicaRegion* region = (SilicaRegion*)region_ptr;
        free(region->data);
        free(region);
    }
}

// Actor system (simplified)
void* silica_actor_spawn(long long initial_state, void* behavior_fn) {
    SilicaActor* actor = (SilicaActor*)malloc(sizeof(SilicaActor));
    if (!actor) return NULL;

    actor->id = 1; // Simple ID
    actor->state = initial_state;
    actor->mailbox = NULL; // Not implementing mailbox for now
    actor->behavior_fn = behavior_fn;

    return actor;
}

void silica_actor_send(void* actor_ptr, long long message) {
    // Simplified - just ignore for now
    (void)actor_ptr;
    (void)message;
}

long long silica_actor_recv(void* actor_ptr) {
    // Simplified - return 0
    (void)actor_ptr;
    return 0;
}

// File I/O (simplified)
SilicaResult silica_read_file(const char* path, size_t path_len) {
    SilicaResult result = {0, NULL};

    // Create a null-terminated copy of the path
    char* path_copy = (char*)malloc(path_len + 1);
    if (!path_copy) return result;

    memcpy(path_copy, path, path_len);
    path_copy[path_len] = '\0';

    FILE* file = fopen(path_copy, "r");
    free(path_copy);

    if (!file) return result;

    // Get file size
    fseek(file, 0, SEEK_END);
    long file_size = ftell(file);
    fseek(file, 0, SEEK_SET);

    if (file_size < 0) {
        fclose(file);
        return result;
    }

    SilicaString* string = (SilicaString*)malloc(sizeof(SilicaString));
    if (!string) {
        fclose(file);
        return result;
    }

    string->data = (char*)malloc(file_size + 1);
    if (!string->data) {
        free(string);
        fclose(file);
        return result;
    }

    size_t bytes_read = fread(string->data, 1, file_size, file);
    string->data[bytes_read] = '\0';
    string->len = bytes_read;

    fclose(file);

    result.success = 1;
    result.data = string;
    return result;
}

SilicaResult silica_write_file(const char* path, size_t path_len,
                              const char* content, size_t content_len) {
    SilicaResult result = {0, NULL};

    // Create a null-terminated copy of the path
    char* path_copy = (char*)malloc(path_len + 1);
    if (!path_copy) return result;

    memcpy(path_copy, path, path_len);
    path_copy[path_len] = '\0';

    FILE* file = fopen(path_copy, "w");
    free(path_copy);

    if (!file) return result;

    size_t bytes_written = fwrite(content, 1, content_len, file);
    fclose(file);

    if (bytes_written == content_len) {
        result.success = 1;
    }

    return result;
}

void silica_free_string(SilicaString* string) {
    if (string) {
        free(string->data);
        free(string);
    }
}

// Process execution (simplified)
ProcessResult* silica_exec_command(const char* cmd, size_t cmd_len) {
    ProcessResult* result = (ProcessResult*)malloc(sizeof(ProcessResult));
    if (!result) return NULL;

    // Initialize with error state
    result->success = 0;
    result->exit_code = -1;
    result->stdout.data = NULL;
    result->stdout.len = 0;
    result->stderr.data = NULL;
    result->stderr.len = 0;

    // For now, just return error - implementing full process execution
    // would require platform-specific code and is complex

    return result;
}

void silica_free_process_result(ProcessResult* result) {
    if (result) {
        silica_free_string(&result->stdout);
        silica_free_string(&result->stderr);
        free(result);
    }
}

// Print functions have been migrated to Rust (runtime.rs)
// This C runtime now only contains memory management, actors, and I/O functions
