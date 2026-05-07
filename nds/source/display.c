#include "display.h"

#include <nds.h>

#include <stdbool.h>
#include <stdio.h>

#include "backlight.h"
#include "display_policy.h"

static ds_controller_display_policy_t display_policy;
static bool bottom_backlight_on;

static void send_backlight_command(uint32_t command) {
    pxiSendAndReceive(PxiChannel_User0, command);
}

static void set_bottom_backlight(bool enabled) {
    if (bottom_backlight_on == enabled) {
        return;
    }

    send_backlight_command(enabled ? DS_CONTROLLER_BACKLIGHT_BOTTOM_ON
                                   : DS_CONTROLLER_BACKLIGHT_BOTTOM_OFF);
    bottom_backlight_on = enabled;
}

static void apply_screen_power(void) {
    if (ds_controller_display_policy_bottom_on(&display_policy)) {
        set_bottom_backlight(true);
    } else {
        set_bottom_backlight(false);
    }
}

void ds_controller_display_init(void) {
    consoleDemoInit();
    pxiWaitRemote(PxiChannel_User0);
    send_backlight_command(DS_CONTROLLER_BACKLIGHT_TOP_OFF);
    bottom_backlight_on = true;
    ds_controller_display_clear();
    ds_controller_display_policy_init(&display_policy);
    apply_screen_power();
}

void ds_controller_display_clear(void) {
    consoleSetWindow(NULL, 0, 0, 32, 24);
    consoleClear();
    consoleSetWindow(NULL, 2, 2, 28, 20);
    iprintf("\x1b[0;0H");
}

void ds_controller_display_update(uint32_t keys_down) {
    ds_controller_display_policy_update(&display_policy, (keys_down & KEY_TOUCH) != 0);
    apply_screen_power();
}
