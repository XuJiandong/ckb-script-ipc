#include "ckb-script-ipc.h"
#include "ckb_syscall_apis.h"

#define MAX_VQL_LEN 10

#define CHECK2(cond, code)                                                                                     \
    do {                                                                                                       \
        if (!(cond)) {                                                                                         \
            err = code;                                                                                        \
            if (err != 1 && err != 2) printf("checking failed on %s:%d, code = %d", __FILE__, __LINE__, code); \
            goto exit;                                                                                         \
        }                                                                                                      \
    } while (0)

#define CHECK(_code)                                                                                           \
    do {                                                                                                       \
        int code = (_code);                                                                                    \
        if (code != 0) {                                                                                       \
            err = code;                                                                                        \
            if (err != 1 && err != 2) printf("checking failed on %s:%d, code = %d", __FILE__, __LINE__, code); \
            goto exit;                                                                                         \
        }                                                                                                      \
    } while (0)

typedef struct CSIContext {
    void* malloc_ctx;
    CSIMalloc malloc;
    CSIFree free;
    CSIPanic panic;
} CSIContext;

CSIContext g_csi_context;

typedef struct CSIMallocFixedContext {
    void* buf;
    size_t len;
    bool freed;
} CSIMallocFixedContext;

CSIMallocFixedContext g_csi_malloc_context;

#define PANIC(e) g_csi_context.panic(e)
int csi_vlq_encode(void* buf, size_t len, uint64_t value, size_t* out_len);
int csi_vlq_decode(const void* buf, size_t len, uint64_t* value, size_t* out_len);
int csi_read_next_vlq(CSIReader* reader, uint64_t* value);

void csi_init_fixed_memory(void* buf, size_t len) {
    g_csi_malloc_context.buf = buf;
    g_csi_malloc_context.len = len;
    g_csi_malloc_context.freed = true;
}

void csi_init_malloc(CSIMalloc malloc, CSIFree free) {
    g_csi_context.malloc = malloc;
    g_csi_context.free = free;
}

void csi_init_panic(CSIPanic panic) { g_csi_context.panic = panic; }

void* csi_malloc_on_fixed(size_t len) {
    if (!g_csi_malloc_context.freed) {
        PANIC(CSI_ERROR_MALLOC);
    }
    if (g_csi_malloc_context.len < len) {
        PANIC(CSI_ERROR_MALLOC_TOO_LARGE);
    }
    g_csi_malloc_context.freed = false;
    return g_csi_malloc_context.buf;
}

void csi_free_on_fixed(void* ptr) {
    if (g_csi_malloc_context.freed) {
        PANIC(CSI_ERROR_DOUBLE_FREE);
    }
    if (g_csi_malloc_context.buf != ptr) {
        PANIC(CSI_ERROR_FREE_WRONG_PTR);
    }
    g_csi_malloc_context.freed = true;
}

int csi_read_pipe(void* ctx, void* buf, size_t len, size_t* read_len) {
    *read_len = len;
    return ckb_read((uint64_t)ctx, buf, read_len);
}

int csi_write_pipe(void* ctx, const void* buf, size_t len, size_t* written_len) {
    *written_len = len;
    return ckb_write((uint64_t)ctx, buf, written_len);
}

int new_pipe_reader(uint64_t fd, CSIReader* reader) {
    if (fd % 2 != 0) {
        return CSI_ERROR_INVALID_FD;
    }
    reader->ctx = (void*)fd;
    reader->read = csi_read_pipe;
    return 0;
}

int new_pipe_writer(uint64_t fd, CSIWriter* writer) {
    if (fd % 2 != 1) {
        return CSI_ERROR_INVALID_FD;
    }
    writer->ctx = (void*)fd;
    writer->write = csi_write_pipe;
    return 0;
}

int csi_send_request(CSIChannel* channel, const CSIRequestPacket* request) {
    int err = 0;
    uint8_t buf[MAX_VQL_LEN];
    size_t len = MAX_VQL_LEN;
    size_t written_len = 0;

    err = csi_vlq_encode(buf, len, request->version, &len);
    CHECK(err);
    err = channel->writer.write(channel->writer.ctx, buf, len, &written_len);
    CHECK(err);
    CHECK2(written_len == len, CSI_ERROR_SEND_REQUEST);

    len = 16;
    err = csi_vlq_encode(buf, len, request->method_id, &len);
    CHECK(err);
    written_len = 0;
    err = channel->writer.write(channel->writer.ctx, buf, len, &written_len);
    CHECK(err);
    CHECK2(written_len == len, CSI_ERROR_SEND_REQUEST);

    len = 16;
    err = csi_vlq_encode(buf, len, request->payload_len, &len);
    CHECK(err);
    written_len = 0;
    err = channel->writer.write(channel->writer.ctx, buf, len, &written_len);
    CHECK(err);
    CHECK2(written_len == len, CSI_ERROR_SEND_REQUEST);

    len = request->payload_len;
    written_len = 0;
    err = channel->writer.write(channel->writer.ctx, request->payload, len, &written_len);
    CHECK(err);
    CHECK2(written_len == len, CSI_ERROR_SEND_REQUEST);
exit:
    return err;
}

int csi_send_response(CSIChannel* channel, const CSIResponsePacket* response) {
    int err = 0;
    uint8_t buf[MAX_VQL_LEN];
    size_t len = MAX_VQL_LEN;
    size_t written_len = 0;

    err = csi_vlq_encode(buf, len, response->version, &len);
    CHECK(err);
    err = channel->writer.write(channel->writer.ctx, buf, len, &written_len);
    CHECK(err);
    CHECK2(written_len == len, CSI_ERROR_SEND_RESPONSE);

    len = sizeof(buf);
    written_len = 0;
    err = csi_vlq_encode(buf, len, response->error_code, &len);
    CHECK(err);
    err = channel->writer.write(channel->writer.ctx, buf, len, &written_len);
    CHECK(err);
    CHECK2(written_len == len, CSI_ERROR_SEND_RESPONSE);

    len = sizeof(buf);
    written_len = 0;
    err = csi_vlq_encode(buf, len, response->payload_len, &len);
    CHECK(err);
    err = channel->writer.write(channel->writer.ctx, buf, len, &written_len);
    CHECK(err);
    CHECK2(written_len == len, CSI_ERROR_SEND_RESPONSE);

    len = response->payload_len;
    written_len = 0;
    err = channel->writer.write(channel->writer.ctx, response->payload, len, &written_len);
    CHECK(err);
    CHECK2(written_len == len, CSI_ERROR_SEND_RESPONSE);

exit:
    return err;
}

int csi_receive_request(CSIChannel* channel, CSIRequestPacket* request) {
    int err = 0;
    err = csi_read_next_vlq(&channel->reader, &request->version);
    CHECK(err);
    err = csi_read_next_vlq(&channel->reader, &request->method_id);
    CHECK(err);
    err = csi_read_next_vlq(&channel->reader, &request->payload_len);
    CHECK(err);

    request->payload = g_csi_context.malloc(request->payload_len);
    size_t read_len = 0;
    err = channel->reader.read(channel->reader.ctx, request->payload, request->payload_len, &read_len);
    CHECK(err);
    CHECK2(read_len == request->payload_len, CSI_ERROR_RECEIVE_REQUEST);
exit:
    return err;
}

int csi_receive_response(CSIChannel* channel, CSIResponsePacket* response) {
    int err = 0;
    err = csi_read_next_vlq(&channel->reader, &response->version);
    CHECK(err);
    err = csi_read_next_vlq(&channel->reader, &response->error_code);
    CHECK(err);
    err = csi_read_next_vlq(&channel->reader, &response->payload_len);
    CHECK(err);

    response->payload = g_csi_context.malloc(response->payload_len);
    size_t read_len = 0;
    err = channel->reader.read(channel->reader.ctx, response->payload, response->payload_len, &read_len);
    CHECK(err);
    CHECK2(read_len == response->payload_len, CSI_ERROR_RECEIVE_RESPONSE);
exit:
    return err;
}

int csi_send_error_code(CSIChannel* channel, int error_code) { return -1; }

int csi_read_next_vlq(CSIReader* reader, uint64_t* value) {
    int err = 0;
    uint8_t peek;
    uint8_t buf[MAX_VQL_LEN];
    size_t buf_len = 0;

    while (1) {
        size_t read_len = 0;
        int err = reader->read(reader->ctx, &peek, 1, &read_len);
        CHECK(err);
        if (buf_len >= sizeof(buf)) {
            return CSI_ERROR_READ_VLQ;
        }
        buf[buf_len++] = peek;

        if ((peek & 0x80) == 0) {
            break;
        }
    }
    size_t read_len = 0;
    err = csi_vlq_decode(buf, buf_len, value, &read_len);
    CHECK(err);
    if (read_len != buf_len) {
        return CSI_ERROR_READ_VLQ;
    }
exit:
    return err;
}

int csi_vlq_encode(void* buf, size_t len, uint64_t value, size_t* out_len) {
    uint8_t* buffer = buf;
    size_t written = 0;

    do {
        if (written >= len) {
            return CSI_ERROR_VQL;
        }

        uint8_t byte = value & 0x7F;
        value >>= 7;

        if (value != 0) {
            byte |= 0x80;
        }

        buffer[written++] = byte;
    } while (value != 0);

    *out_len = written;
    return 0;
}

int csi_vlq_decode(const void* buf, size_t len, uint64_t* value, size_t* out_len) {
    const uint8_t* bytes = (const uint8_t*)buf;
    uint64_t result = 0;
    size_t shift = 0;
    size_t read = 0;

    while (read < len) {
        uint8_t byte = bytes[read++];
        result |= ((uint64_t)(byte & 0x7F) << shift);

        if ((byte & 0x80) == 0) {
            *value = result;
            *out_len = read;
            return 0;
        }

        shift += 7;
        if (shift >= 64) {
            return CSI_ERROR_VQL;
        }
    }

    return CSI_ERROR_VQL;
}
