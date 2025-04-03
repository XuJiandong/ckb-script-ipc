// CSI: Ckb Script Ipc

#ifndef __CKB_SCRIPT_IPC_H__
#define __CKB_SCRIPT_IPC_H__
#include <stdint.h>

/**
 * Error Code.
 * The functions in this library return this error code to indicate success or failure.
 */
typedef enum CSIErrorCode {
    CSI_SUCCESS = 0,
    CSI_ERROR_INVALID_REQUEST = 50,
    CSI_ERROR_VQL,
    CSI_ERROR_MALLOC,
    CSI_ERROR_MALLOC_TOO_LARGE,
    CSI_ERROR_DOUBLE_FREE,
    CSI_ERROR_FREE_WRONG_PTR,
    CSI_ERROR_INVALID_FD,
    CSI_ERROR_READ_VLQ,
    CSI_ERROR_RECEIVE_REQUEST,
    CSI_ERROR_RECEIVE_RESPONSE,
    CSI_ERROR_SEND_REQUEST,
    CSI_ERROR_SEND_RESPONSE,
} CSIErrorCode;

typedef void* (*CSIMalloc)(size_t len);
typedef void (*CSIFree)(void* ptr);
typedef void (*CSIPanic)(int exit_code);

/**
 * Initialize a fixed-size memory allocator.
 * This allocator uses a pre-allocated buffer for all memory operations.
 *
 * @param buf Pointer to the pre-allocated memory buffer
 * @param len Size of the pre-allocated buffer in bytes
 *
 * @note This function is mutually exclusive with csi_init_malloc().
 *       Only one memory allocation strategy can be active at a time.
 */
void csi_init_fixed_memory(void* buf, size_t len);

/**
 * Initialize a custom memory allocator.
 * This allows using external memory management functions.
 *
 * @param malloc Function pointer to custom memory allocation function
 * @param free Function pointer to custom memory deallocation function
 *
 * @note This function is mutually exclusive with csi_init_fixed_memory().
 *       Only one memory allocation strategy can be active at a time.
 */
void csi_init_malloc(CSIMalloc malloc, CSIFree free);

/**
 * Initialize a custom panic handler function.
 * This allows the user to define custom behavior when a panic occurs.
 *
 * @param panic Function pointer to the custom panic handler
 *
 * @note The exit code passed to the panic handler indicates the reason
 *       for the panic.
 */
void csi_init_panic(CSIPanic panic);

/**
 * Read data interface
 * @param ctx: Implementation-specific context (similar to 'this' in C++)
 * @param buf: Destination buffer to store read data
 * @param len: length of `buf`
 * @param read_len: Number of bytes actually read (output parameter)
 * @return 0 for success, non-zero for failure
 */
typedef int (*CSIRead)(void* ctx, void* buf, size_t len, size_t* read_len);

typedef struct CSIReader {
    void* ctx;
    CSIRead read;
} CSIReader;

/**
 * Write data interface
 * @param ctx: Implementation-specific context (similar to 'this' in C++)
 * @param buf: Source buffer containing data to write
 * @param len: length of `buf`
 * @param write_len: Number of bytes actually written (output parameter)
 * @return 0 for success, non-zero for failure
 */
typedef int (*CSIWrite)(void* ctx, const void* buf, size_t len, size_t* written_len);

typedef struct CSIWriter {
    void* ctx;
    CSIWrite write;
} CSIWriter;

typedef struct CSIRequestPacket {
    uint64_t version;
    uint64_t method_id;
    size_t payload_len;
    void* payload;
} CSIRequestPacket;

typedef struct CSIResponsePacket {
    uint64_t version;
    uint64_t error_code;
    size_t payload_len;
    void* payload;
} CSIResponsePacket;

typedef struct CSIChannel {
    CSIReader reader;
    CSIWriter writer;
} CSIChannel;

/**
 * Sends a request and waits for a response on the given channel.
 *
 * @param channel: The channel to send/receive on
 * @param request: The request packet to send
 * @param response: Pointer to store the received response packet
 * @return 0 for success, non-zero for failure
 *
 * Note: The caller is responsible for freeing(csi_free) the response's payload
 * when it's no longer needed.
 */
int csi_call(CSIChannel* channel, const CSIRequestPacket* request, CSIResponsePacket* response);

int csi_spawn_server(uint64_t index, uint64_t source, const char* argv[], int argc, CSIChannel* client_channel);
int csi_spawn_cell_server(void* code_hash, uint64_t hash_type, const char* argv[], int argc,
                          CSIChannel* client_channel);

/**
 * Callback function type for handling IPC requests in the server.
 *
 * @param request: The incoming request packet containing the client's request data
 * @param response: The response packet to be filled with the server's response
 * @return 0 for success, non-zero for failure
 *
 * This callback is called by csi_run_server for each incoming request.
 * The implementation should process the request and populate the response packet.
 */
typedef int (*CSIServe)(const CSIRequestPacket* request, CSIResponsePacket* response);

/**
 * Runs the IPC server loop, processing incoming requests using the provided callback.
 *
 * @param serve: The callback function that will handle each incoming request
 * @return 0 for success, non-zero for failure
 *
 * This function enters an infinite loop that:
 * 1. Receives requests from clients
 * 2. Calls the provided serve callback to process each request
 * 3. Sends responses back to clients
 *
 * The server will continue running until an error occurs or the process is terminated.
 */
int csi_run_server(CSIServe serve);

#endif
