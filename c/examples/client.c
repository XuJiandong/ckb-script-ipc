#include <stdlib.h>
#include <stdio.h>
#include "ckb_consts.h"
#include "ckb_syscalls.h"
#include "ckb_script_ipc.h"

static uint8_t g_payload_buf[1024];
static uint8_t g_io_buf[2048];

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

    for (size_t i = 0; i < 1; i++) {
        // step 1: craft a request packet
        CSIRequestPacket request = {0};
        request.version = 0;
        request.method_id = 1;
        // Initialize request payload fields - not all requests require payload data
        // When no payload is needed, set length to 0 and pointer to NULL
        request.payload_len = 0;
        request.payload = NULL;

        // step 2: send the request and receive the response to finish a call
        CSIResponsePacket response;
        err = csi_call(&channel, &request, &response);
        if (err) {
            printf("failed to call server: %d\n", err);
            return err;
        }

        // use or verify the returned response
        for (size_t j = 0; j < response.payload_len; j++) {
            uint8_t value = ((uint8_t*)response.payload)[j];
            if (value != 42) {
                printf("value is not 42: %d\n", value);
                return 1;
            }
        }

        // step 3: free the response payload
        csi_client_free_response_payload(&response);
    }
}
