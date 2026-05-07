#ifndef DS_CONTROLLER_DISPLAY_POLICY_H
#define DS_CONTROLLER_DISPLAY_POLICY_H

#include <stdbool.h>
#include <stdint.h>

enum {
    DS_CONTROLLER_DISPLAY_WAKE_FRAMES = 60 * 5,
};

typedef struct {
    uint32_t bottom_wake_frames;
} ds_controller_display_policy_t;

void ds_controller_display_policy_init(ds_controller_display_policy_t *policy);
void ds_controller_display_policy_update(ds_controller_display_policy_t *policy, bool touch_pressed);
void ds_controller_display_policy_wake_bottom(ds_controller_display_policy_t *policy);
bool ds_controller_display_policy_bottom_on(const ds_controller_display_policy_t *policy);

#endif
