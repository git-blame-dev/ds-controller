#ifndef DS_CONTROLLER_PROTOCOL_H
#define DS_CONTROLLER_PROTOCOL_H

#include <stdint.h>

#define DS_CONTROLLER_PACKET_SIZE 16u
#define DS_CONTROLLER_PROTOCOL_VERSION 1u
#define DS_CONTROLLER_MESSAGE_CONTROLLER_STATE 1u

typedef struct {
    uint8_t bytes[DS_CONTROLLER_PACKET_SIZE];
} ds_controller_packet_t;

typedef enum {
    DS_CONTROLLER_BUTTON_A = 1u << 0,
    DS_CONTROLLER_BUTTON_B = 1u << 1,
    DS_CONTROLLER_BUTTON_X = 1u << 2,
    DS_CONTROLLER_BUTTON_Y = 1u << 3,
    DS_CONTROLLER_BUTTON_L = 1u << 4,
    DS_CONTROLLER_BUTTON_R = 1u << 5,
    DS_CONTROLLER_BUTTON_START = 1u << 6,
    DS_CONTROLLER_BUTTON_SELECT = 1u << 7,
    DS_CONTROLLER_BUTTON_DPAD_UP = 1u << 8,
    DS_CONTROLLER_BUTTON_DPAD_DOWN = 1u << 9,
    DS_CONTROLLER_BUTTON_DPAD_LEFT = 1u << 10,
    DS_CONTROLLER_BUTTON_DPAD_RIGHT = 1u << 11,
} ds_controller_button_t;

void ds_controller_encode_packet(ds_controller_packet_t *packet, uint32_t sequence, uint16_t buttons);

#endif
