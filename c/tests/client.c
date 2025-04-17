#include <stdlib.h>
#include <stdio.h>
#include "ckb_syscalls.h"
#include "ckb_consts.h"
#include "ckb_script_ipc.h"

static uint8_t g_payload_buf[4096];
static uint8_t g_io_buf[1024];

int main() {
    printf("client started");
    csi_init_payload(g_payload_buf, sizeof(g_payload_buf), 2);
    csi_init_iobuf(g_io_buf, sizeof(g_io_buf), 2);

    int err = 0;
    CSIChannel channel;
    err = csi_spawn_server(0, CKB_SOURCE_CELL_DEP, 0, 0, NULL, 0, &channel);
    if (err) {
        printf("failed to spawn server: %d\n", err);
        return err;
    }
    size_t loop_count = 11;
    for (size_t i = 0; i < loop_count; i++) {
        CSIRequestPacket request = {0};
        request.version = 0;
        request.method_id = 1;

        uint64_t sum = 0;
        uint64_t payload_len = i * 97;
        uint8_t payload[payload_len];
        for (size_t j = 0; j < payload_len; j++) {
            payload[j] = (uint8_t)j;
            sum += (uint8_t)j;
        }
        request.payload_len = payload_len;
        request.payload = payload;

        CSIResponsePacket response;
        err = csi_call(&channel, &request, &response);
        if (err) {
            printf("failed to call server: %d\n", err);
            return err;
        }

        uint64_t real_sum = *((uint64_t*)response.payload);
        if (real_sum != sum) {
            printf("The result is wrong: real_sum(%lu) vs sum(%lu)", real_sum, sum);
            return -42;
        }
        csi_client_free_response_payload(&response);
    }
}
