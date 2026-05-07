#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "../source/input.h"
#include "../source/protocol.h"

#define TEST_KEY_A (1u << 0)
#define TEST_KEY_B (1u << 1)
#define TEST_KEY_SELECT (1u << 2)
#define TEST_KEY_START (1u << 3)
#define TEST_KEY_RIGHT (1u << 4)
#define TEST_KEY_LEFT (1u << 5)
#define TEST_KEY_UP (1u << 6)
#define TEST_KEY_DOWN (1u << 7)
#define TEST_KEY_R (1u << 8)
#define TEST_KEY_L (1u << 9)
#define TEST_KEY_X (1u << 10)
#define TEST_KEY_Y (1u << 11)

static int failures = 0;

static void expect_u16(const char *name, uint16_t actual, uint16_t expected) {
    if (actual != expected) {
        printf("FAIL %s: got 0x%04x expected 0x%04x\n", name, actual, expected);
        failures++;
    }
}

static void expect_bytes(const char *name, const uint8_t *actual, const uint8_t *expected, size_t len) {
    if (memcmp(actual, expected, len) != 0) {
        printf("FAIL %s\n", name);
        failures++;
    }
}

static void encodes_rust_golden_vector(void) {
    ds_controller_packet_t packet;
    const uint8_t expected[DS_CONTROLLER_PACKET_SIZE] = {
        'D', 'S', 'C', 'P', 1, 1, 16, 0, 42, 0, 0, 0, 1, 1, 0, 0,
    };

    ds_controller_encode_packet(&packet, 42, DS_CONTROLLER_BUTTON_A | DS_CONTROLLER_BUTTON_DPAD_UP);

    expect_bytes("golden packet", packet.bytes, expected, sizeof(expected));
}

static void encodes_full_sequence_little_endian(void) {
    ds_controller_packet_t packet;
    const uint8_t expected_sequence_bytes[] = {0x12, 0x34, 0x56, 0x78};

    ds_controller_encode_packet(&packet, 0x78563412u, 0);

    expect_bytes("sequence byte order", &packet.bytes[8], expected_sequence_bytes,
                 sizeof(expected_sequence_bytes));
}

static void maps_each_ds_key_to_protocol_button(void) {
    const struct {
        const char *name;
        uint32_t key;
        uint16_t button;
    } cases[] = {
        {"A", TEST_KEY_A, DS_CONTROLLER_BUTTON_A},
        {"B", TEST_KEY_B, DS_CONTROLLER_BUTTON_B},
        {"X", TEST_KEY_X, DS_CONTROLLER_BUTTON_X},
        {"Y", TEST_KEY_Y, DS_CONTROLLER_BUTTON_Y},
        {"L", TEST_KEY_L, DS_CONTROLLER_BUTTON_L},
        {"R", TEST_KEY_R, DS_CONTROLLER_BUTTON_R},
        {"Start", TEST_KEY_START, DS_CONTROLLER_BUTTON_START},
        {"Select", TEST_KEY_SELECT, DS_CONTROLLER_BUTTON_SELECT},
        {"Up", TEST_KEY_UP, DS_CONTROLLER_BUTTON_DPAD_UP},
        {"Down", TEST_KEY_DOWN, DS_CONTROLLER_BUTTON_DPAD_DOWN},
        {"Left", TEST_KEY_LEFT, DS_CONTROLLER_BUTTON_DPAD_LEFT},
        {"Right", TEST_KEY_RIGHT, DS_CONTROLLER_BUTTON_DPAD_RIGHT},
    };

    for (size_t i = 0; i < sizeof(cases) / sizeof(cases[0]); i++) {
        expect_u16(cases[i].name, ds_controller_buttons_from_keys(cases[i].key), cases[i].button);
    }
}

static void maps_combined_keys_without_extra_buttons(void) {
    const uint32_t keys = TEST_KEY_A | TEST_KEY_L | TEST_KEY_START | TEST_KEY_RIGHT;
    const uint16_t expected = DS_CONTROLLER_BUTTON_A | DS_CONTROLLER_BUTTON_L |
                              DS_CONTROLLER_BUTTON_START | DS_CONTROLLER_BUTTON_DPAD_RIGHT;

    expect_u16("combined keys", ds_controller_buttons_from_keys(keys), expected);
}

int main(void) {
    encodes_rust_golden_vector();
    encodes_full_sequence_little_endian();
    maps_each_ds_key_to_protocol_button();
    maps_combined_keys_without_extra_buttons();

    if (failures != 0) {
        printf("%d test(s) failed\n", failures);
        return 1;
    }

    printf("all protocol tests passed\n");
    return 0;
}
