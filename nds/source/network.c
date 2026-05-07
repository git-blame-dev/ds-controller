#include "network.h"

#include "config.h"
#include "display.h"

#include <nds.h>

#include <arpa/inet.h>
#include <dswifi9.h>
#include <netinet/in.h>
#include <sys/socket.h>

#include <stdio.h>
#include <string.h>

static struct sockaddr_in target_addr;

static const char *wifi_status_text(int status) {
    switch (status) {
    case ASSOCSTATUS_DISCONNECTED:
        return "disconnected";
    case ASSOCSTATUS_SEARCHING:
        return "searching";
    case ASSOCSTATUS_ASSOCIATING:
        return "associating";
    case ASSOCSTATUS_ACQUIRINGDHCP:
        return "getting DHCP";
    case ASSOCSTATUS_ASSOCIATED:
        return "associated";
    default:
        return "unknown";
    }
}

int ds_controller_network_init(ds_controller_network_t *network) {
    memset(network, 0, sizeof(*network));
    network->socket_fd = -1;

    if (!Wifi_InitDefault(INIT_ONLY)) {
        network->last_error = -1;
        return -1;
    }

    Wifi_AutoConnect();

    int last_status = -1;
    while (pmMainLoop()) {
        const int status = Wifi_AssocStatus();
        if (status != last_status) {
            iprintf("Wi-Fi: %s\n", wifi_status_text(status));
            last_status = status;
        }

        if (status == ASSOCSTATUS_ASSOCIATED) {
            break;
        }

        if (status == ASSOCSTATUS_CANNOTCONNECT) {
            network->last_error = status;
            return -1;
        }

        swiWaitForVBlank();
        scanKeys();
        const uint32_t down = keysDown();
        ds_controller_display_update(down);
        if (down & KEY_START) {
            network->last_error = -2;
            return -1;
        }
    }

    network->socket_fd = socket(AF_INET, SOCK_DGRAM, 0);
    if (network->socket_fd < 0) {
        network->last_error = network->socket_fd;
        return -1;
    }

    memset(&target_addr, 0, sizeof(target_addr));
    target_addr.sin_family = AF_INET;
    target_addr.sin_port = htons(DS_CONTROLLER_PC_PORT);
    target_addr.sin_addr.s_addr = inet_addr(DS_CONTROLLER_PC_IP);
    if (target_addr.sin_addr.s_addr == INADDR_NONE) {
        network->last_error = -2;
        ds_controller_network_close(network);
        return -1;
    }

    return 0;
}

int ds_controller_network_send(ds_controller_network_t *network, const uint8_t *bytes, size_t len) {
    const int sent = sendto(network->socket_fd, bytes, len, 0, (struct sockaddr *)&target_addr,
                            sizeof(target_addr));
    if (sent < 0 || (size_t)sent != len) {
        network->last_error = sent;
        return -1;
    }

    network->last_error = 0;
    return 0;
}

void ds_controller_network_close(ds_controller_network_t *network) {
    if (network->socket_fd >= 0) {
        closesocket(network->socket_fd);
        network->socket_fd = -1;
    }
}
