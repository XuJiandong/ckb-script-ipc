#include <stdint.h>
#include "ckb_syscalls.h"
#include "ckb_script_ipc.h"

static uint8_t g_payload_buf[4096];
static uint8_t g_io_buf[1024];

static int serve_call_back(const CSIRequestPacket* request, CSIResponsePacket* response) {
    printf("serve callback");
    int err = 0;
    // Only accept requests with method_id == 1
    if (request->method_id != 1) {
        printf("Ignoring request with method_id %lu (expected: 1)\n", request->method_id);
        return 0;
    }
    uint64_t sum = 0;

    for (size_t i = 0; i < request->payload_len; i++) {
        sum += ((uint8_t*)request->payload)[i];
    }

    response->payload_len = 8;
    csi_server_malloc_response_payload(response);
    *((uint64_t*)response->payload) = sum;
    return 0;
}

int main() {
    // initialize the fixed memory allocator
    csi_init_payload(g_payload_buf, sizeof(g_payload_buf), 2);
    csi_init_iobuf(g_io_buf, sizeof(g_io_buf), 2);
    return csi_run_server(serve_call_back);
}
