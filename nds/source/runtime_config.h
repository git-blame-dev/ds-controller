#ifndef DS_CONTROLLER_RUNTIME_CONFIG_H
#define DS_CONTROLLER_RUNTIME_CONFIG_H

#include <stdbool.h>
#include <stdint.h>

#define DS_CONTROLLER_RUNTIME_CONFIG_IP_MAX 16u

typedef struct {
    char pc_ip[DS_CONTROLLER_RUNTIME_CONFIG_IP_MAX];
    uint16_t pc_port;
    bool loaded_from_file;
} ds_controller_runtime_config_t;

typedef enum {
    DS_CONTROLLER_CONFIG_LOAD_DEFAULT = 0,
    DS_CONTROLLER_CONFIG_LOAD_FILE = 1,
    DS_CONTROLLER_CONFIG_LOAD_INVALID = -1,
} ds_controller_config_load_result_t;

void ds_controller_runtime_config_default(ds_controller_runtime_config_t *config);
int ds_controller_runtime_config_parse(ds_controller_runtime_config_t *config, const char *text);
ds_controller_config_load_result_t ds_controller_runtime_config_load(ds_controller_runtime_config_t *config);

#endif
