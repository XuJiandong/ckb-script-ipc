#ifndef __CKB_SCRIPT_IPC_H__
#define __CKB_SCRIPT_IPC_H__

#include <stdint.h>

/**
 * Error Code.
 * The functions in this library return this error code to indicate success or failure.
 */
typedef enum CSIErrorCode {
    CSI_SUCCESS = 0,
    CSI_ERROR_INVALID_REQUEST = 1,
} CSIErrorCode;

typedef void* (*CSIMalloc)(void* ctx, size_t len);
typedef void (*CSIFree)(void* ctx, void* ptr);

/**
 * Initialize the csi library by set malloc and free functions
 * @param malloc: malloc function
 * @param free: free function
 */
void csi_init(void* malloc_ctx, CSIMalloc malloc, CSIFree free);

typedef struct CSIContext {
    void* malloc_ctx;
    CSIMalloc malloc;
    CSIFree free;
} CSIContext;

extern CSIContext g_csi_context;

typedef struct CSIMallocFixedContext {
    void* buf;
    size_t len;
    bool freed;
} CSIMallocFixedContext;

/**
 * Allocates memory from a fixed-size buffer.
 *
 * This is a specialized memory allocator that works with a pre-allocated buffer,
 * which can be either stack-allocated or a global variable. It's useful in
 * environments where dynamic memory allocation is restricted or unavailable.
 *
 * @param ctx Pointer to the context(`CSIMallocFixedContext`)
 * @param len Number of bytes to allocate
 * @return Pointer to allocated memory, or NULL if allocation fails
 */
void* csi_malloc_on_fixed(void* ctx, size_t len);

/**
 * Frees memory previously allocated by csi_malloc_on_fixed.
 *
 * @param ctx Pointer to the context(`CSIMallocFixedContext`)
 * @param ptr Pointer to the memory to free
 */
void csi_free_on_fixed(void* ctx, void* ptr);

/**
 * Read data interface
 * @param ctx: Implementation-specific context (similar to 'this' in C++)
 * @param buf: Destination buffer to store read data
 * @param len: length of `buf`
 * @param read_len: Number of bytes actually read (output parameter)
 * @return 0 for success, non-zero for failure
 */
typedef int (*CSIRead)(void* ctx, uint8_t* buf, size_t len, size_t* read_len);

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
typedef int (*CSIWrite)(void* ctx, const uint8_t* buf, size_t len, size_t* write_len);

typedef struct CSIWriter {
    void* ctx;
    CSIWrite write;
} CSIWriter;

int new_pipe_reader(uint64_t fd, CSIReader* reader);
int new_pipe_writer(uint64_t fd, CSIWriter* writer);

/**
 * Read a next vlq value from the reader.
 * @param reader: The reader to read from
 * @param value: Pointer to store the read value
 * @return 0 for success, non-zero for failure
 */
int csi_read_next_vlq(CSIReader* reader, uint64_t* value);

typedef struct CSIRequestPacket {
    uint64_t version;
    uint64_t method_id;
    size_t payload_len;
    uint8_t* payload;
} CSIRequestPacket;

/**
 * Read a request packet from the channel.
 * @param reader: The reader to read from
 * @param request: Pointer to store the received request packet
 * @return 0 for success, non-zero for failure
 * Note, the caller is responsible for freeing(csi_free) the request's payload when it's no longer needed.
 */
int csi_read_request(CSIReader* reader, CSIRequestPacket* request);
/**
 * Write a request packet to the channel.
 * @param writer: The writer to write to
 * @param request: The request packet to write
 * @return 0 for success, non-zero for failure
 */
int csi_write_request(CSIWriter* writer, const CSIRequestPacket* request);

typedef struct CSIResponsePacket {
    uint64_t version;
    uint64_t error_code;
    size_t payload_len;
    uint8_t* payload;
} CSIResponsePacket;

/**
 * Read a response packet from the channel.
 * @param reader: The reader to read from
 * @param response: Pointer to store the received response packet
 * @return 0 for success, non-zero for failure
 * Note, the caller is responsible for freeing(csi_free) the response's payload when it's no longer needed.
 */
int csi_read_response(CSIRead* reader, CSIResponsePacket* response);
/**
 * Write a response packet to the channel.
 * @param writer: The writer to write to
 * @param response: The response packet to write
 * @return 0 for success, non-zero for failure
 */
int csi_write_response(CSIWrite* writer, const CSIResponsePacket* response);

typedef struct CSIChannel {
    CSIReader reader;
    CSIWriter writer;
} CSIChannel;

int csi_send_request(CSIChannel* channel, const CSIRequestPacket* request);
int csi_send_response(CSIChannel* channel, const CSIResponsePacket* response);
/**
 * Receives a request packet from the channel.
 *
 * @param channel: The channel to receive from
 * @param request: Pointer to store the received request packet
 * @return 0 for success, non-zero for failure
 *
 * Note: The caller is responsible for freeing(csi_free) the request's payload when it's no longer needed.
 */
int csi_receive_request(CSIChannel* channel, CSIRequestPacket* request);
/**
 * Receives a response packet from the channel.
 *
 * @param channel: The channel to receive from
 * @param response: Pointer to store the received response packet
 * @return 0 for success, non-zero for failure
 *
 * Note: The caller is responsible for freeing(csi_free) the response's payload when it's no longer needed.
 */
int csi_receive_response(CSIChannel* channel, CSIResponsePacket* response);
int csi_send_error_code(CSIChannel* channel, int error_code);
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

/**
 * Encodes a 64-bit unsigned integer into a VLQ (Variable-Length Quantity) format.
 *
 * @param buf: Output buffer to store the encoded bytes
 * @param len: Maximum length of the output buffer
 * @param value: The 64-bit value to encode
 * @param out_len: Pointer to store the number of bytes written
 * @return 0 for success, non-zero for failure
 */
int csi_vlq_encode(uint8_t* buf, size_t len, uint64_t value, size_t* out_len);

/**
 * Decodes a VLQ (Variable-Length Quantity) encoded buffer into a 64-bit unsigned integer.
 *
 * @param buf: Input buffer containing the VLQ encoded bytes
 * @param len: Length of the input buffer
 * @param value: Pointer to store the decoded value
 * @param out_len: Pointer to store the number of bytes consumed
 * @return 0 for success, non-zero for failure
 */
int csi_vlq_decode(const uint8_t* buf, size_t len, uint64_t* value, size_t* out_len);

int csi_spawn_server(uint64_t index, uint64_t source, const char* argv[], int argc, uint64_t fds[2]);
int csi_spawn_cell_server(uint8_t* code_hash, uint64_t hash_type, const char* argv[], int argc, uint64_t fds[2]);

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
