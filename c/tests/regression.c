#include "ckb_syscalls.h"
// Include the implementation to exercise allocator and decoder boundaries directly.
#include "../ckb_script_ipc.c"

#define REQUIRE(condition)                                    \
    do {                                                      \
        if (!(condition)) {                                   \
            printf("regression failed at line %d", __LINE__); \
            ckb_exit(1);                                      \
        }                                                     \
    } while (0)

typedef struct Input {
    const uint8_t* data;
    size_t len;
    size_t pos;
    int error;
    size_t max_read;
} Input;

static int read_input(void* ctx, void* buf, size_t len, size_t* read_len) {
    Input* input = ctx;
    *read_len = 0;
    if (input->pos == input->len) {
        return input->error;
    }
    size_t available = input->len - input->pos;
    *read_len = len < available ? len : available;
    if (input->max_read != 0 && *read_len > input->max_read) {
        *read_len = input->max_read;
    }
    memcpy(buf, input->data + input->pos, *read_len);
    input->pos += *read_len;
    return 0;
}

static int write_output(void* ctx, const void* buf, size_t len, size_t* written_len) {
    (void)buf;
    *written_len = len;
    return ctx == NULL ? 0 : CSI_ERROR_SEND_REQUEST;
}

static int flush_output(void* ctx) {
    (void)ctx;
    return 0;
}

static uint64_t custom_storage[4][128];
static bool custom_used[4];
static size_t custom_allocations;
static size_t custom_frees;

static void* custom_malloc(size_t len) {
    REQUIRE(len > 0 && len <= sizeof(custom_storage[0]));
    for (size_t i = 0; i < 4; i++) {
        if (!custom_used[i]) {
            custom_used[i] = true;
            custom_allocations++;
            return custom_storage[i];
        }
    }
    return NULL;
}

static void custom_free(void* ptr) {
    if (ptr == NULL) {
        return;
    }
    for (size_t i = 0; i < 4; i++) {
        if (ptr == custom_storage[i]) {
            REQUIRE(custom_used[i]);
            custom_used[i] = false;
            custom_frees++;
            return;
        }
    }
    REQUIRE(false);
}

static void test_custom_allocator_and_empty_response(void) {
    custom_allocations = 0;
    custom_frees = 0;
    csi_init_malloc(custom_malloc, custom_free);
    const uint8_t packets[] = {0, 0, 1, 42, 0, 0, 0};
    Input input = {.data = packets, .len = sizeof(packets)};
    CSIReader reader = {.ctx = &input, .read = read_input};
    CSIWriter writer = {.write = write_output, .flush = flush_output};
    CSIChannel channel = {0};
    new_buf_reader(reader, &channel.reader);
    new_buf_writer(writer, &channel.writer);
    REQUIRE(custom_allocations == 2);

    CSIRequestPacket request = {0};
    CSIResponsePacket response;
    memset(&response, 0xAA, sizeof(response));
    REQUIRE(csi_call(&channel, &request, &response) == 0);
    REQUIRE(response.payload_len == 1 && ((uint8_t*)response.payload)[0] == 42);
    csi_client_free_response_payload(&response);
    REQUIRE(response.payload == NULL && response.payload_len == 0);
    csi_client_free_response_payload(&response);
    REQUIRE(custom_frees == 1);

    memset(&response, 0xAA, sizeof(response));
    REQUIRE(csi_call(&channel, &request, &response) == 0);
    REQUIRE(response.payload_len == 0 && response.payload == NULL);
    csi_client_free_response_payload(&response);
    csi_free_channel(&channel);
    REQUIRE(custom_allocations == 3 && custom_frees == 3);
}

static void test_fixed_allocator(void) {
    uint64_t storage[64][2];
    FixedAllocator allocator;
    fixed_allocator_init(&allocator, storage, sizeof(storage), 64);
    REQUIRE(fixed_allocator_malloc(&allocator, sizeof(storage[0]) + 1) == NULL);
    for (size_t i = 0; i < 64; i++) {
        REQUIRE(fixed_allocator_malloc(&allocator, sizeof(storage[0])) == storage[i]);
        storage[i][0] = i;
    }
    REQUIRE(fixed_allocator_malloc(&allocator, 1) == NULL);
    const size_t slots[] = {31, 32, 63};
    for (size_t i = 0; i < 3; i++) {
        fixed_allocator_free(&allocator, storage[slots[i]]);
    }
    for (size_t i = 0; i < 3; i++) {
        REQUIRE(fixed_allocator_malloc(&allocator, 1) == storage[slots[i]]);
    }
    for (size_t i = 0; i < 64; i++) {
        REQUIRE(storage[i][0] == i);
        fixed_allocator_free(&allocator, storage[i]);
    }
    for (size_t i = 0; i < 64; i++) {
        REQUIRE(fixed_allocator_malloc(&allocator, 1) == storage[i]);
    }
}

static void test_read_errors(void) {
    uint8_t continuation = 0x80;
    Input input = {.error = CSI_ERROR_INVALID_FD};
    CSIChannel channel = {.reader = {.ctx = &input, .read = read_input},
                          .writer = {.write = write_output, .flush = flush_output}};
    uint64_t value = 42;
    REQUIRE(csi_read_vlq(&channel.reader, &value) == CSI_ERROR_INVALID_FD);
    REQUIRE(value == 42);
    input.data = &continuation;
    input.len = 1;
    REQUIRE(csi_read_vlq(&channel.reader, &value) == CSI_ERROR_INVALID_FD);
    input.error = 0;
    input.pos = 0;
    REQUIRE(csi_read_vlq(&channel.reader, &value) == CSI_ERROR_READ_VLQ);
    REQUIRE(csi_read_vlq(&channel.reader, &value) == CSI_ERROR_READ_VLQ);

    input.error = CSI_ERROR_INVALID_FD;
    CSIResponsePacket response;
    memset(&response, 0xAA, sizeof(response));
    REQUIRE(csi_receive_response(&channel, &response) == CSI_ERROR_INVALID_FD);
    REQUIRE(response.payload == NULL);
    csi_client_free_response_payload(&response);

    CSIRequestPacket request = {0};
    channel.writer.ctx = &input;
    memset(&response, 0xAA, sizeof(response));
    REQUIRE(csi_call(&channel, &request, &response) == CSI_ERROR_SEND_REQUEST);
    REQUIRE(response.payload == NULL);
    csi_client_free_response_payload(&response);
}

static void test_truncated_response_payload(void) {
    custom_allocations = 0;
    custom_frees = 0;
    csi_init_malloc(custom_malloc, custom_free);
    const uint8_t packet[] = {0, 0, 2, 42, 43};
    for (int buffered = 0; buffered < 2; buffered++) {
        for (size_t available = 0; available <= 2; available++) {
            Input input = {.data = packet, .len = 3 + available, .max_read = 1};
            CSIReader reader = {.ctx = &input, .read = read_input};
            CSIWriter writer = {.write = write_output, .flush = flush_output};
            CSIChannel channel = {.reader = reader, .writer = writer};
            if (buffered) {
                new_buf_reader(reader, &channel.reader);
                new_buf_writer(writer, &channel.writer);
            }
            CSIRequestPacket request = {0};
            CSIResponsePacket response = {0};
            int expected = available == 2 ? CSI_SUCCESS : CSI_ERROR_UNEXPECTED_EOF;
            REQUIRE(csi_call(&channel, &request, &response) == expected);
            REQUIRE(input.pos == input.len);
            REQUIRE(response.payload_len == 2 && response.payload != NULL);
            REQUIRE(memcmp(response.payload, packet + 3, available) == 0);
            csi_client_free_response_payload(&response);
            if (buffered) {
                csi_free_channel(&channel);
            }
            REQUIRE(custom_allocations == custom_frees);
        }
    }

    Input input = {.error = CSI_ERROR_INVALID_FD};
    CSIReader reader = {.ctx = &input, .read = read_input};
    uint8_t byte = 0;
    REQUIRE(csi_read_exact(&reader, &byte, 0) == CSI_SUCCESS);
    REQUIRE(csi_read_exact(&reader, &byte, 1) == CSI_ERROR_INVALID_FD);
}

static void test_vlq_overflow(void) {
    uint8_t encoded[10];
    memset(encoded, 0x80, sizeof(encoded));
    uint64_t value;
    size_t consumed;
    for (unsigned terminal = 2; terminal <= 0x7F; terminal++) {
        encoded[9] = terminal;
        REQUIRE(csi_vlq_decode(encoded, sizeof(encoded), &value, &consumed) == CSI_ERROR_VLQ);
    }
    encoded[9] = 1;
    REQUIRE(csi_vlq_decode(encoded, sizeof(encoded), &value, &consumed) == 0);
    REQUIRE(value == ((uint64_t)1 << 63) && consumed == 10);
    memset(encoded, 0xFF, 9);
    REQUIRE(csi_vlq_decode(encoded, sizeof(encoded), &value, &consumed) == 0);
    REQUIRE(value == UINT64_MAX);

    uint8_t unterminated[11];
    memset(unterminated, 0x80, sizeof(unterminated));
    Input input = {.data = unterminated, .len = sizeof(unterminated)};
    CSIReader reader = {.ctx = &input, .read = read_input};
    REQUIRE(csi_read_vlq(&reader, &value) == CSI_ERROR_READ_VLQ);
    REQUIRE(input.pos == 10);
}

static void expected_panic(int code) {
#if defined(TEST_ZERO_BLOCKS)
    ckb_exit(code == CSI_ERROR_FA_INVALID_BLOCK_COUNT ? 0 : 1);
#else
    ckb_exit(code == CSI_ERROR_IOBUF_TOO_SMALL ? 0 : 1);
#endif
}

int main(void) {
    static uint64_t io_storage[128];
#if defined(TEST_SMALL_IOBUF)
    csi_init_panic(expected_panic);
    csi_init_iobuf(io_storage, sizeof(io_storage), 32);
    return 1;
#elif defined(TEST_HEADER_ONLY_IOBUF)
    static uint64_t header_storage[sizeof(CSIBuffer) * 32 / sizeof(uint64_t)];
    csi_init_panic(expected_panic);
    csi_init_iobuf(header_storage, sizeof(header_storage), 32);
    return 1;
#elif defined(TEST_ZERO_BLOCKS)
    csi_init_panic(expected_panic);
    csi_init_iobuf(io_storage, sizeof(io_storage), 0);
    return 1;
#else
    test_custom_allocator_and_empty_response();
    test_fixed_allocator();
    test_read_errors();
    test_truncated_response_payload();
    test_vlq_overflow();
    // Switching from fixed allocation must not retain its block size or free callback.
    csi_init_iobuf(io_storage, sizeof(io_storage), 2);
    test_custom_allocator_and_empty_response();
    return 0;
#endif
}
