#include <nds.h>

#include <stdbool.h>
#include <stdio.h>

#include "config.h"
#include "display.h"
#include "input.h"
#include "network.h"
#include "protocol.h"

static void send_controller_packet(ds_controller_network_t *network, ds_controller_packet_t *packet,
                                   uint32_t *sequence) {
    const uint16_t buttons = ds_controller_buttons_from_keys(keysHeld());
    ds_controller_encode_packet(packet, (*sequence)++, buttons);
    (void)ds_controller_network_send(network, packet->bytes, sizeof(packet->bytes));
}

static void print_status(void) {
    ds_controller_display_clear();
    iprintf("DS Controller\n");
    iprintf("Target: %s:%d\n", DS_CONTROLLER_PC_IP, DS_CONTROLLER_PC_PORT);
    iprintf("Connected\n");
    iprintf("Sending controller input\n");
    iprintf("Touch wakes this screen\n");
    iprintf("Power off to stop\n");
}

static bool run_controller(void) {
    ds_controller_display_init();

    iprintf("DS Controller\n");
    iprintf("Target: %s:%d\n", DS_CONTROLLER_PC_IP, DS_CONTROLLER_PC_PORT);
    iprintf("Connecting Wi-Fi...\n");

    ds_controller_network_t network;
    if (ds_controller_network_init(&network) != 0) {
        iprintf("Wi-Fi failed: %d\n", network.last_error);
        iprintf("Press Start to retry.\n");
        while (pmMainLoop()) {
            swiWaitForVBlank();
            scanKeys();
            const uint32_t down = keysDown();
            ds_controller_display_update(down);
            if (down & KEY_START) {
                return true;
            }
        }
        return false;
    }

    print_status();

    uint32_t sequence = 0;
    ds_controller_packet_t packet;

    while (pmMainLoop()) {
        swiWaitForVBlank();
        scanKeys();

        ds_controller_display_update(keysDown());
        send_controller_packet(&network, &packet, &sequence);
    }

    ds_controller_network_close(&network);
    return false;
}

int main(void) {
    while (pmMainLoop()) {
        if (!run_controller()) {
            break;
        }
    }

    return 0;
}
