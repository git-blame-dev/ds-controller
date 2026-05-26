#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>

#include "../source/send_policy.h"

static int failures = 0;

static void expect_bool(const char *name, bool actual, bool expected) {
    if (actual != expected) {
        printf("FAIL %s: got %d expected %d\n", name, actual ? 1 : 0, expected ? 1 : 0);
        failures++;
    }
}

static bool should_send(ds_controller_send_policy_t *policy, uint16_t buttons) {
    return ds_controller_send_policy_should_send(policy, buttons);
}

static void initial_neutral_sends_nothing(void) {
    ds_controller_send_policy_t policy;
    ds_controller_send_policy_init(&policy);

    expect_bool("initial neutral sends nothing", should_send(&policy, 0), false);
}

static void neutral_idle_sends_nothing(void) {
    ds_controller_send_policy_t policy;
    ds_controller_send_policy_init(&policy);

    should_send(&policy, 0);

    expect_bool("neutral idle sends nothing", should_send(&policy, 0), false);
}

static void neutral_to_pressed_sends(void) {
    ds_controller_send_policy_t policy;
    ds_controller_send_policy_init(&policy);

    should_send(&policy, 0);

    expect_bool("neutral to pressed sends", should_send(&policy, 1u), true);
}

static void same_pressed_sends_every_frame(void) {
    ds_controller_send_policy_t policy;
    ds_controller_send_policy_init(&policy);

    expect_bool("first pressed frame sends", should_send(&policy, 1u), true);
    expect_bool("same pressed frame sends", should_send(&policy, 1u), true);
    expect_bool("same pressed frame keeps sending", should_send(&policy, 1u), true);
}

static void pressed_to_different_pressed_sends(void) {
    ds_controller_send_policy_t policy;
    ds_controller_send_policy_init(&policy);

    should_send(&policy, 1u);

    expect_bool("different pressed sends", should_send(&policy, 2u), true);
}

static void pressed_to_neutral_sends_three_neutral_packets(void) {
    ds_controller_send_policy_t policy;
    ds_controller_send_policy_init(&policy);

    should_send(&policy, 1u);

    expect_bool("neutral release packet 1", should_send(&policy, 0), true);
    expect_bool("neutral release packet 2", should_send(&policy, 0), true);
    expect_bool("neutral release packet 3", should_send(&policy, 0), true);
    expect_bool("neutral release packet 4 stops", should_send(&policy, 0), false);
}

static void press_during_neutral_burst_cancels_remaining_burst(void) {
    ds_controller_send_policy_t policy;
    ds_controller_send_policy_init(&policy);

    should_send(&policy, 1u);
    expect_bool("neutral burst starts", should_send(&policy, 0), true);

    expect_bool("press during neutral burst sends", should_send(&policy, 2u), true);
    expect_bool("held press after burst cancel sends", should_send(&policy, 2u), true);

    expect_bool("new release packet 1", should_send(&policy, 0), true);
    expect_bool("new release packet 2", should_send(&policy, 0), true);
    expect_bool("new release packet 3", should_send(&policy, 0), true);
    expect_bool("new release stops", should_send(&policy, 0), false);
}

int main(void) {
    initial_neutral_sends_nothing();
    neutral_idle_sends_nothing();
    neutral_to_pressed_sends();
    same_pressed_sends_every_frame();
    pressed_to_different_pressed_sends();
    pressed_to_neutral_sends_three_neutral_packets();
    press_during_neutral_burst_cancels_remaining_burst();

    if (failures != 0) {
        printf("%d test(s) failed\n", failures);
        return 1;
    }

    printf("all send policy tests passed\n");
    return 0;
}
