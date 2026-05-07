#include "input.h"

#include "protocol.h"

#ifdef DS_CONTROLLER_HOST_TEST
#define KEY_A (1u << 0)
#define KEY_B (1u << 1)
#define KEY_SELECT (1u << 2)
#define KEY_START (1u << 3)
#define KEY_RIGHT (1u << 4)
#define KEY_LEFT (1u << 5)
#define KEY_UP (1u << 6)
#define KEY_DOWN (1u << 7)
#define KEY_R (1u << 8)
#define KEY_L (1u << 9)
#define KEY_X (1u << 10)
#define KEY_Y (1u << 11)
#else
#include <nds.h>
#endif

uint16_t ds_controller_buttons_from_keys(uint32_t keys_held) {
    uint16_t buttons = 0;

    if (keys_held & KEY_A) {
        buttons |= DS_CONTROLLER_BUTTON_A;
    }
    if (keys_held & KEY_B) {
        buttons |= DS_CONTROLLER_BUTTON_B;
    }
    if (keys_held & KEY_X) {
        buttons |= DS_CONTROLLER_BUTTON_X;
    }
    if (keys_held & KEY_Y) {
        buttons |= DS_CONTROLLER_BUTTON_Y;
    }
    if (keys_held & KEY_L) {
        buttons |= DS_CONTROLLER_BUTTON_L;
    }
    if (keys_held & KEY_R) {
        buttons |= DS_CONTROLLER_BUTTON_R;
    }
    if (keys_held & KEY_START) {
        buttons |= DS_CONTROLLER_BUTTON_START;
    }
    if (keys_held & KEY_SELECT) {
        buttons |= DS_CONTROLLER_BUTTON_SELECT;
    }
    if (keys_held & KEY_UP) {
        buttons |= DS_CONTROLLER_BUTTON_DPAD_UP;
    }
    if (keys_held & KEY_DOWN) {
        buttons |= DS_CONTROLLER_BUTTON_DPAD_DOWN;
    }
    if (keys_held & KEY_LEFT) {
        buttons |= DS_CONTROLLER_BUTTON_DPAD_LEFT;
    }
    if (keys_held & KEY_RIGHT) {
        buttons |= DS_CONTROLLER_BUTTON_DPAD_RIGHT;
    }

    return buttons;
}
