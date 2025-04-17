#include <stdlib.h>
#include "ckb_consts.h"
#include "ckb_syscalls.h"
#include "ckb_script_ipc.h"

static uint8_t g_payload_buf[1024];
static uint8_t g_io_buf[2048];

static int serve_call_back(const CSIRequestPacket* request, CSIResponsePacket* response) {
    printf("serve callback");
    int err = 0;
    // Only accept requests with method_id == 1
    if (request->method_id != 1) {
        printf("Ignoring request with method_id %d (expected: 1)\n", request->method_id);
        return 0;
    }

    // set target payload length
    response->payload_len = 3;
    // allocate memory for the payload
    csi_server_malloc_response_payload(response);
    // fill the payload with some data
    for (int i = 0; i < response->payload_len; i++) {
        ((uint8_t*)response->payload)[i] = 42;
    }
    return 0;
}

int main() {
    // initialize the fixed memory allocator
    csi_init_payload(g_payload_buf, sizeof(g_payload_buf), 2);
    csi_init_iobuf(g_io_buf, sizeof(g_io_buf), 2);
    return csi_run_server(serve_call_back);
}
