#include <stdbool.h>
#include <stdio.h>

#include "../source/display_policy.h"

static int failures = 0;

static void expect_bool(const char *name, bool actual, bool expected) {
    if (actual != expected) {
        printf("FAIL %s: got %d expected %d\n", name, actual ? 1 : 0, expected ? 1 : 0);
        failures++;
    }
}

static void bottom_screen_starts_off(void) {
    ds_controller_display_policy_t policy;
    ds_controller_display_policy_init(&policy);

    expect_bool("bottom starts off", ds_controller_display_policy_bottom_on(&policy), false);
}

static void touch_wakes_bottom_screen(void) {
    ds_controller_display_policy_t policy;
    ds_controller_display_policy_init(&policy);

    ds_controller_display_policy_update(&policy, true);

    expect_bool("touch wakes bottom", ds_controller_display_policy_bottom_on(&policy), true);
}

static void normal_buttons_do_not_wake_bottom_screen(void) {
    ds_controller_display_policy_t policy;
    ds_controller_display_policy_init(&policy);

    ds_controller_display_policy_update(&policy, false);

    expect_bool("buttons do not wake bottom", ds_controller_display_policy_bottom_on(&policy), false);
}

static void bottom_screen_turns_off_after_timeout(void) {
    ds_controller_display_policy_t policy;
    ds_controller_display_policy_init(&policy);
    ds_controller_display_policy_update(&policy, true);

    for (int i = 0; i < DS_CONTROLLER_DISPLAY_WAKE_FRAMES; i++) {
        expect_bool("bottom remains on before timeout", ds_controller_display_policy_bottom_on(&policy), true);
        ds_controller_display_policy_update(&policy, false);
    }

    expect_bool("bottom turns off after timeout", ds_controller_display_policy_bottom_on(&policy), false);
}

static void repeated_touch_extends_timeout(void) {
    ds_controller_display_policy_t policy;
    ds_controller_display_policy_init(&policy);
    ds_controller_display_policy_update(&policy, true);

    for (int i = 0; i < DS_CONTROLLER_DISPLAY_WAKE_FRAMES - 1; i++) {
        ds_controller_display_policy_update(&policy, false);
    }

    ds_controller_display_policy_update(&policy, true);
    ds_controller_display_policy_update(&policy, false);

    expect_bool("repeated touch extends timeout", ds_controller_display_policy_bottom_on(&policy), true);
}

int main(void) {
    bottom_screen_starts_off();
    touch_wakes_bottom_screen();
    normal_buttons_do_not_wake_bottom_screen();
    bottom_screen_turns_off_after_timeout();
    repeated_touch_extends_timeout();

    if (failures != 0) {
        printf("%d test(s) failed\n", failures);
        return 1;
    }

    printf("all display policy tests passed\n");
    return 0;
}
