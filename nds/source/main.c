#include <nds.h>

#include <stdio.h>

#include "config.h"
#include "input.h"
#include "network.h"
#include "protocol.h"

int main(void) {
    consoleDemoInit();

    iprintf("DS Controller\n");
    iprintf("Target: %s:%d\n", DS_CONTROLLER_PC_IP, DS_CONTROLLER_PC_PORT);
    iprintf("Connecting Wi-Fi...\n");

    ds_controller_network_t network;
    if (ds_controller_network_init(&network) != 0) {
        iprintf("Wi-Fi failed: %d\n", network.last_error);
        iprintf("Press Start to exit.\n");
        while (pmMainLoop()) {
            swiWaitForVBlank();
            scanKeys();
            if (keysDown() & KEY_START) {
                break;
            }
        }
        return 1;
    }

    iprintf("Connected.\n");
    iprintf("Hold buttons to send.\n");

    uint32_t sequence = 0;
    uint32_t packets_sent = 0;
    uint32_t send_errors = 0;
    ds_controller_packet_t packet;

    while (pmMainLoop()) {
        swiWaitForVBlank();
        scanKeys();

        const uint32_t held = keysHeld();
        const uint16_t buttons = ds_controller_buttons_from_keys(held);
        ds_controller_encode_packet(&packet, sequence++, buttons);

        if (ds_controller_network_send(&network, packet.bytes, sizeof(packet.bytes)) == 0) {
            packets_sent++;
        } else {
            send_errors++;
        }

        if ((sequence % 60u) == 0u) {
            consoleClear();
            iprintf("DS Controller\n");
            iprintf("Target: %s:%d\n", DS_CONTROLLER_PC_IP, DS_CONTROLLER_PC_PORT);
            iprintf("Packets: %lu\n", (unsigned long)packets_sent);
            iprintf("Errors: %lu\n", (unsigned long)send_errors);
            iprintf("Last err: %d\n", network.last_error);
            iprintf("Reset/power off to stop\n");
        }
    }

    ds_controller_network_close(&network);
    return 0;
}
