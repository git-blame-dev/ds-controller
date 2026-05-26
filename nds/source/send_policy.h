#ifndef DS_CONTROLLER_SEND_POLICY_H
#define DS_CONTROLLER_SEND_POLICY_H

#include <stdbool.h>
#include <stdint.h>

enum {
    DS_CONTROLLER_NEUTRAL_RELEASE_SENDS = 3,
};

typedef struct {
    uint16_t last_buttons;
    uint8_t neutral_sends_remaining;
} ds_controller_send_policy_t;

void ds_controller_send_policy_init(ds_controller_send_policy_t *policy);
bool ds_controller_send_policy_should_send(ds_controller_send_policy_t *policy, uint16_t buttons);

#endif
