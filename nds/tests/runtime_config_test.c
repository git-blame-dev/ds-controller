#include "runtime_config.h"

#include <assert.h>
#include <string.h>

static void parses_valid_config(void) {
    ds_controller_runtime_config_t config;
    assert(ds_controller_runtime_config_parse(&config, "pc_ip=192.168.1.50\npc_port=26761\n") == 0);
    assert(strcmp(config.pc_ip, "192.168.1.50") == 0);
    assert(config.pc_port == 26761u);
    assert(config.loaded_from_file);
}

static void uses_build_defaults(void) {
    ds_controller_runtime_config_t config;
    ds_controller_runtime_config_default(&config);
    assert(strcmp(config.pc_ip, "192.0.2.1") == 0);
    assert(config.pc_port == 26760u);
    assert(!config.loaded_from_file);
}

static void allows_comments_and_spacing(void) {
    ds_controller_runtime_config_t config;
    assert(ds_controller_runtime_config_parse(&config,
                                              "# DS Controller\n"
                                              " pc_ip = 10.0.0.8 # receiver\n"
                                              " pc_port = 26760\n") == 0);
    assert(strcmp(config.pc_ip, "10.0.0.8") == 0);
    assert(config.pc_port == 26760u);
}

static void rejects_missing_values(void) {
    ds_controller_runtime_config_t config;
    assert(ds_controller_runtime_config_parse(&config, "pc_ip=192.168.1.50\n") != 0);
    assert(ds_controller_runtime_config_parse(&config, "pc_port=26760\n") != 0);
}

static void rejects_invalid_values(void) {
    ds_controller_runtime_config_t config;
    assert(ds_controller_runtime_config_parse(&config, "pc_ip=999.168.1.50\npc_port=26760\n") != 0);
    assert(ds_controller_runtime_config_parse(&config, "pc_ip=255.255.255.255\npc_port=26760\n") != 0);
    assert(ds_controller_runtime_config_parse(&config, "pc_ip=192.168.1.50\npc_port=0\n") != 0);
    assert(ds_controller_runtime_config_parse(&config, "pc_ip=192.168.1.50\npc_port=65536\n") != 0);
    assert(ds_controller_runtime_config_parse(&config, "pc_ip=192.168.1.50\nfoo=bar\npc_port=26760\n") != 0);
}

int main(void) {
    parses_valid_config();
    uses_build_defaults();
    allows_comments_and_spacing();
    rejects_missing_values();
    rejects_invalid_values();
    return 0;
}
