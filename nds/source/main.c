#include <nds.h>

#include <stdbool.h>
#include <stdio.h>

#include "config.h"
#include "display.h"
#include "input.h"
#include "network.h"
#include "protocol.h"
#include "runtime_config.h"

#define RETRY_DELAY_FRAMES (60u * 2u)

static void send_controller_packet(ds_controller_network_t *network, ds_controller_packet_t *packet,
                                   uint32_t *sequence) {
    const uint16_t buttons = ds_controller_buttons_from_keys(keysHeld());
    ds_controller_encode_packet(packet, (*sequence)++, buttons);
    (void)ds_controller_network_send(network, packet->bytes, sizeof(packet->bytes));
}

static void print_config_load_status(ds_controller_config_load_result_t load_result) {
    switch (load_result) {
    case DS_CONTROLLER_CONFIG_LOAD_FILE:
        iprintf("Config: file\n");
        break;
    case DS_CONTROLLER_CONFIG_LOAD_INVALID:
        iprintf("Config invalid; using defaults\n");
        break;
    case DS_CONTROLLER_CONFIG_LOAD_DEFAULT:
    default:
        iprintf("Config: defaults\n");
        break;
    }
}

static void print_status(const ds_controller_runtime_config_t *config) {
    ds_controller_display_clear();
    iprintf("DS Controller\n");
    iprintf("Target: %s:%u\n", config->pc_ip, config->pc_port);
    iprintf("Connected\n");
    iprintf("Sending controller input\n");
    iprintf("Touch wakes this screen\n");
    iprintf("Power off to stop\n");
}

static bool run_controller(const ds_controller_runtime_config_t *config,
                           ds_controller_config_load_result_t load_result) {
    ds_controller_display_init();

    iprintf("DS Controller\n");
    print_config_load_status(load_result);
    iprintf("Target: %s:%u\n", config->pc_ip, config->pc_port);
    iprintf("Connecting Wi-Fi...\n");

    ds_controller_network_t network;
    if (ds_controller_network_init(&network, config) != 0) {
        iprintf("Wi-Fi failed: %d\n", network.last_error);
        iprintf("Retrying...\n");
        for (uint32_t frame = 0; frame < RETRY_DELAY_FRAMES; frame++) {
            if (!pmMainLoop()) {
                return false;
            }

            swiWaitForVBlank();
            scanKeys();
            ds_controller_display_update(keysDown());
        }
        return true;
    }

    print_status(config);

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
    ds_controller_runtime_config_t config;
    const ds_controller_config_load_result_t load_result = ds_controller_runtime_config_load(&config);

    while (pmMainLoop()) {
        if (!run_controller(&config, load_result)) {
            break;
        }
    }

    return 0;
}
