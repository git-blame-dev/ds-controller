#include "network.h"

#include "config.h"

#include <arpa/inet.h>
#include <dswifi9.h>
#include <netinet/in.h>
#include <sys/socket.h>

#include <string.h>

static struct sockaddr_in target_addr;

int ds_controller_network_init(ds_controller_network_t *network) {
    memset(network, 0, sizeof(*network));
    network->socket_fd = -1;

    if (!Wifi_InitDefault(WFC_CONNECT)) {
        network->last_error = -1;
        return -1;
    }

    network->socket_fd = socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP);
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
