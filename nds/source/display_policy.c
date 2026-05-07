#include "display_policy.h"

void ds_controller_display_policy_init(ds_controller_display_policy_t *policy) {
    policy->bottom_wake_frames = 0;
}

void ds_controller_display_policy_update(ds_controller_display_policy_t *policy, bool touch_pressed) {
    if (touch_pressed) {
        ds_controller_display_policy_wake_bottom(policy);
        return;
    }

    if (policy->bottom_wake_frames > 0) {
        policy->bottom_wake_frames--;
    }
}

void ds_controller_display_policy_wake_bottom(ds_controller_display_policy_t *policy) {
    policy->bottom_wake_frames = DS_CONTROLLER_DISPLAY_WAKE_FRAMES;
}

bool ds_controller_display_policy_bottom_on(const ds_controller_display_policy_t *policy) {
    return policy->bottom_wake_frames > 0;
}
