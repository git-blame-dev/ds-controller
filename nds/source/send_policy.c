#include "send_policy.h"

void ds_controller_send_policy_init(ds_controller_send_policy_t *policy) {
    policy->last_buttons = 0;
    policy->neutral_sends_remaining = 0;
}

bool ds_controller_send_policy_should_send(ds_controller_send_policy_t *policy, uint16_t buttons) {
    if (buttons != 0) {
        policy->last_buttons = buttons;
        policy->neutral_sends_remaining = 0;
        return true;
    }

    if (policy->last_buttons != 0) {
        policy->last_buttons = 0;
        policy->neutral_sends_remaining = DS_CONTROLLER_NEUTRAL_RELEASE_SENDS;
    }

    if (policy->neutral_sends_remaining > 0) {
        policy->neutral_sends_remaining--;
        return true;
    }

    return false;
}
