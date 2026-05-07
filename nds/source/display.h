#ifndef DS_CONTROLLER_DISPLAY_H
#define DS_CONTROLLER_DISPLAY_H

#include <stdint.h>

void ds_controller_display_init(void);
void ds_controller_display_clear(void);
void ds_controller_display_update(uint32_t keys_down);

#endif
