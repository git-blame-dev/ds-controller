#include "protocol.h"

_Static_assert(sizeof(ds_controller_packet_t) == DS_CONTROLLER_PACKET_SIZE,
               "controller packet must stay 16 bytes");

void ds_controller_encode_packet(ds_controller_packet_t *packet, uint32_t sequence, uint16_t buttons) {
    packet->bytes[0] = 'D';
    packet->bytes[1] = 'S';
    packet->bytes[2] = 'C';
    packet->bytes[3] = 'P';
    packet->bytes[4] = DS_CONTROLLER_PROTOCOL_VERSION;
    packet->bytes[5] = DS_CONTROLLER_MESSAGE_CONTROLLER_STATE;
    packet->bytes[6] = DS_CONTROLLER_PACKET_SIZE;
    packet->bytes[7] = 0;

    packet->bytes[8] = (uint8_t)(sequence & 0xffu);
    packet->bytes[9] = (uint8_t)((sequence >> 8) & 0xffu);
    packet->bytes[10] = (uint8_t)((sequence >> 16) & 0xffu);
    packet->bytes[11] = (uint8_t)((sequence >> 24) & 0xffu);

    packet->bytes[12] = (uint8_t)(buttons & 0xffu);
    packet->bytes[13] = (uint8_t)((buttons >> 8) & 0xffu);
    packet->bytes[14] = 0;
    packet->bytes[15] = 0;
}
