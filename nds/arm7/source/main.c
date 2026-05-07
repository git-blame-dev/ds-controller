#include <nds.h>

#include <calico/nds/arm7/pmic.h>
#include <stdbool.h>

#include "backlight.h"

static Thread backlight_thread;
static uint8_t backlight_thread_stack[1024] __attribute__((aligned(8)));

static void set_backlight(uint8_t mask, bool enabled) {
    spiLock();
    uint8_t control = pmicReadRegister(PmicReg_Control);
    if (enabled) {
        control |= mask;
    } else {
        control &= (uint8_t)~mask;
    }
    pmicWriteRegister(PmicReg_Control, control);
    spiUnlock();
}

static int backlight_thread_main(void *arg) {
    (void)arg;

    Mailbox mailbox;
    uint32_t slots[4];
    mailboxPrepare(&mailbox, slots, sizeof(slots) / sizeof(slots[0]));
    pxiSetMailbox(PxiChannel_User0, &mailbox);

    for (;;) {
        const uint32_t command = mailboxRecv(&mailbox);

        switch (command) {
        case DS_CONTROLLER_BACKLIGHT_TOP_OFF:
            set_backlight(PMIC_CTRL_LCD_BL_TOP, false);
            break;
        case DS_CONTROLLER_BACKLIGHT_TOP_ON:
            set_backlight(PMIC_CTRL_LCD_BL_TOP, true);
            break;
        case DS_CONTROLLER_BACKLIGHT_BOTTOM_OFF:
            set_backlight(PMIC_CTRL_LCD_BL_BOTTOM, false);
            break;
        case DS_CONTROLLER_BACKLIGHT_BOTTOM_ON:
            set_backlight(PMIC_CTRL_LCD_BL_BOTTOM, true);
            break;
        default:
            break;
        }

        pxiReply(PxiChannel_User0, 0);
    }

    return 0;
}

int main(void) {
    envReadNvramSettings();
    keypadStartExtServer();

    lcdSetIrqMask(DISPSTAT_IE_ALL, DISPSTAT_IE_VBLANK);
    irqEnable(IRQ_VBLANK);

    rtcInit();
    rtcSyncTime();

    pmInit();

    touchInit();
    touchStartServer(80, MAIN_THREAD_PRIO);

    wlmgrStartServer(MAIN_THREAD_PRIO - 8);

    threadPrepare(&backlight_thread, backlight_thread_main, NULL,
                  &backlight_thread_stack[sizeof(backlight_thread_stack)], MAIN_THREAD_PRIO);
    threadStart(&backlight_thread);

    while (pmMainLoop()) {
        threadWaitForVBlank();
    }

    return 0;
}
