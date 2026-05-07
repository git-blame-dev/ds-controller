#ifndef DS_CONTROLLER_NETWORK_H
#define DS_CONTROLLER_NETWORK_H

#include <stddef.h>
#include <stdint.h>

typedef struct {
    int socket_fd;
    int last_error;
} ds_controller_network_t;

int ds_controller_network_init(ds_controller_network_t *network);
int ds_controller_network_send(ds_controller_network_t *network, const uint8_t *bytes, size_t len);
void ds_controller_network_close(ds_controller_network_t *network);

#endif
