The C implementation of ckb-script-ipc.

## Build and Integrate
```bash
make all
```

or integrate [ckb_script_ipc.c](./ckb_script_ipc.c) and [ckb_script_ipc.h](./ckb_script_ipc.h) into C projects.

## How to Use

### Client Side Implementation
```C
// 1. Initialize memory allocation
// You can choose between fixed memory or custom allocator:
// Option A: Fixed memory buffer
uint8_t g_malloc_buf[BUFFER_SIZE] __attribute__((aligned(8)));
csi_init_fixed_memory(g_malloc_buf, sizeof(g_malloc_buf));

// Option B: Custom allocator
// csi_init_malloc(malloc, free);

// 2. Spawn server process
// Option A: Using low-level spawn
CSIChannel channel;
csi_spawn_server(
    index,
    CKB_SOURCE_CELL_DEP,
    0,
    0,
    NULL,
    0,
    &channel
);

// Option B: Using cell-based spawn
// csi_spawn_cell_server(code_hash, hash_type, argv, argc, &channel);

// 3. Prepare and send request
CSIRequestPacket request = {
    .version = 0,       // protocol version
    .method_id = 0,     // your method identifier
    .payload_len = 0,   // length of payload data
    .payload = NULL     // pointer to payload data
};
// Fill request with your data

// 4. Make the call and receive response
CSIResponsePacket response;
int ret = csi_call(&channel, &request, &response);
if (ret == CSI_SUCCESS) {
    // Check response.error_code for operation status
    // Process response.payload (if any)
}

// 5. Clean up
csi_client_free_response_payload(&response);
```

### Server Side Implementation
```C
static int serve_callback(const CSIRequestPacket* request, CSIResponsePacket* response) {
    // Process the request based on method_id
    switch (request->method_id) {
        case YOUR_METHOD:
            // Handle the method
            response->version = 0;
            response->error_code = CSI_SUCCESS;
            response->payload_len = result_size;

            // Allocate response payload if needed
            csi_server_malloc_response_payload(response);
            // Fill response->payload with your data
            break;

        default:
            response->error_code = CSI_ERROR_INVALID_REQUEST;
            return CSI_ERROR_INVALID_REQUEST;
    }

    return CSI_SUCCESS;
}

uint8_t g_malloc_buf[BUFFER_SIZE];

int main() {
    // Initialize memory management
    // Option A: Fixed memory buffer
    csi_init_fixed_memory(g_malloc_buf, sizeof(g_malloc_buf));
    // Option B: Custom allocator
    // csi_init_malloc(malloc, free);

    // Start server loop
    return csi_run_server(serve_callback);
}
```

See detailed [client example](./examples/client.c) and [server example](./examples/server.c).

## Memory Allocation
This project requires dynamic memory allocation for handling request and response payloads. Since many on-chain C scripts don't have access to `malloc` by default, we provide a simple fixed memory allocator.

Initialize the fixed memory allocator with:
```C
void csi_init_fixed_memory(void* buf, size_t len);
```

The fixed memory allocator has the following characteristics:
- Maximum allocation size: `len/2`
- Maximum number of concurrent allocations: 2
- Memory must be 2-byte aligned

Memory management must be handled in two specific places:

On the client side, you must free the response payload after processing:
```C
// Make the call
csi_call(&channel, &request, &response);

// Process response data
// ...

// Free the response payload
csi_client_free_response_payload(&response);
```

On the server side, you must allocate the response payload before filling data:
```C
static int serve_callback(const CSIRequestPacket* request, CSIResponsePacket* response) {
    // Set response metadata
    response->version = 0;
    response->payload_len = calculated_size;

    // Allocate memory for response payload
    csi_server_malloc_response_payload(response);

    // Fill response payload with data
    // ...

    return 0;
}
```
The fixed allocator is designed for simple IPC scenarios - if you need more complex memory management, consider implementing a custom allocator.
