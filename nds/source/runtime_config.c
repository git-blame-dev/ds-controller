#include "runtime_config.h"

#include "config.h"

#include <ctype.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifndef DS_CONTROLLER_HOST_TEST
#include <fat.h>
#endif

#define DS_CONTROLLER_CONFIG_MAX_BYTES 512u
#define DS_CONTROLLER_CONFIG_MAX_PATH 192u

static char *trim(char *value) {
    while (isspace((unsigned char)*value)) {
        value++;
    }

    char *end = value + strlen(value);
    while (end > value && isspace((unsigned char)end[-1])) {
        end--;
    }
    *end = '\0';

    return value;
}

static bool parse_ipv4(const char *value, char *out, size_t out_len) {
    unsigned long octets[4] = {0};
    const char *cursor = value;

    for (size_t index = 0; index < 4u; index++) {
        if (!isdigit((unsigned char)*cursor)) {
            return false;
        }

        char *end = NULL;
        octets[index] = strtoul(cursor, &end, 10);
        if (end == cursor || octets[index] > 255u) {
            return false;
        }

        if (index < 3u) {
            if (*end != '.') {
                return false;
            }
            cursor = end + 1;
        } else if (*end != '\0') {
            return false;
        }
    }

    if (octets[0] == 255u && octets[1] == 255u && octets[2] == 255u && octets[3] == 255u) {
        return false;
    }

    const int written = snprintf(out, out_len, "%lu.%lu.%lu.%lu", octets[0], octets[1], octets[2], octets[3]);
    return written > 0 && (size_t)written < out_len;
}

static bool parse_port(const char *value, uint16_t *out) {
    if (!isdigit((unsigned char)*value)) {
        return false;
    }

    char *end = NULL;
    const unsigned long port = strtoul(value, &end, 10);
    if (end == value || *end != '\0' || port == 0u || port > 65535u) {
        return false;
    }

    *out = (uint16_t)port;
    return true;
}

static bool config_path_for_rom(char *out, size_t out_len, const char *rom_path) {
    if (rom_path == NULL || *rom_path == '\0') {
        return false;
    }

    const char *last_slash = strrchr(rom_path, '/');
    const char *last_backslash = strrchr(rom_path, '\\');
    if (last_backslash != NULL && (last_slash == NULL || last_backslash > last_slash)) {
        last_slash = last_backslash;
    }

    if (last_slash == NULL) {
        const int written = snprintf(out, out_len, "%s", DS_CONTROLLER_CONFIG_FILE_NAME);
        return written > 0 && (size_t)written < out_len;
    }

    const size_t dir_len = (size_t)(last_slash - rom_path) + 1u;
    if (dir_len + strlen(DS_CONTROLLER_CONFIG_FILE_NAME) + 1u > out_len) {
        return false;
    }

    memcpy(out, rom_path, dir_len);
    strcpy(out + dir_len, DS_CONTROLLER_CONFIG_FILE_NAME);
    return true;
}

#ifdef DS_CONTROLLER_HOST_TEST
bool ds_controller_runtime_config_path_for_rom(char *out, size_t out_len, const char *rom_path) {
    return config_path_for_rom(out, out_len, rom_path);
}
#endif

void ds_controller_runtime_config_default(ds_controller_runtime_config_t *config) {
    snprintf(config->pc_ip, sizeof(config->pc_ip), "%s", DS_CONTROLLER_PC_IP);
    config->pc_port = (uint16_t)DS_CONTROLLER_PC_PORT;
    config->loaded_from_file = false;
}

int ds_controller_runtime_config_parse(ds_controller_runtime_config_t *config, const char *text) {
    ds_controller_runtime_config_t parsed;
    ds_controller_runtime_config_default(&parsed);
    parsed.loaded_from_file = true;

    char buffer[DS_CONTROLLER_CONFIG_MAX_BYTES + 1u];
    const size_t text_len = strlen(text);
    if (text_len > DS_CONTROLLER_CONFIG_MAX_BYTES) {
        return -1;
    }
    memcpy(buffer, text, text_len + 1u);

    bool saw_ip = false;
    bool saw_port = false;
    char *line = buffer;
    while (line != NULL) {
        char *next = strchr(line, '\n');
        if (next != NULL) {
            *next = '\0';
            next++;
        }

        char *comment = strchr(line, '#');
        if (comment != NULL) {
            *comment = '\0';
        }

        char *entry = trim(line);
        if (*entry != '\0') {
            char *separator = strchr(entry, '=');
            if (separator == NULL) {
                return -1;
            }

            *separator = '\0';
            char *key = trim(entry);
            char *value = trim(separator + 1);

            if (strcmp(key, "pc_ip") == 0) {
                if (!parse_ipv4(value, parsed.pc_ip, sizeof(parsed.pc_ip))) {
                    return -1;
                }
                saw_ip = true;
            } else if (strcmp(key, "pc_port") == 0) {
                if (!parse_port(value, &parsed.pc_port)) {
                    return -1;
                }
                saw_port = true;
            } else {
                return -1;
            }
        }

        line = next;
    }

    if (!saw_ip || !saw_port) {
        return -1;
    }

    *config = parsed;
    return 0;
}

ds_controller_config_load_result_t ds_controller_runtime_config_load(ds_controller_runtime_config_t *config,
                                                                     const char *rom_path) {
    ds_controller_runtime_config_default(config);

#ifdef DS_CONTROLLER_HOST_TEST
    (void)rom_path;
    return DS_CONTROLLER_CONFIG_LOAD_DEFAULT;
#else
    if (!fatInitDefault()) {
        return DS_CONTROLLER_CONFIG_LOAD_DEFAULT;
    }

    char buffer[DS_CONTROLLER_CONFIG_MAX_BYTES + 1u];
    char same_dir_path[DS_CONTROLLER_CONFIG_MAX_PATH];
    const char *paths[] = {
        NULL,
        DS_CONTROLLER_CONFIG_DIR_PATH,
        DS_CONTROLLER_CONFIG_ROOT_PATH,
    };

    if (config_path_for_rom(same_dir_path, sizeof(same_dir_path), rom_path)) {
        paths[0] = same_dir_path;
    }

    bool found_invalid_config = false;
    for (size_t index = 0; index < sizeof(paths) / sizeof(paths[0]); index++) {
        if (paths[index] == NULL) {
            continue;
        }

        FILE *file = fopen(paths[index], "rb");
        if (file == NULL) {
            continue;
        }

        const size_t bytes_read = fread(buffer, 1u, DS_CONTROLLER_CONFIG_MAX_BYTES + 1u, file);
        const int read_error = ferror(file);
        fclose(file);

        if (read_error != 0 || bytes_read > DS_CONTROLLER_CONFIG_MAX_BYTES) {
            found_invalid_config = true;
            continue;
        }

        buffer[bytes_read] = '\0';
        if (ds_controller_runtime_config_parse(config, buffer) == 0) {
            return DS_CONTROLLER_CONFIG_LOAD_FILE;
        }
        found_invalid_config = true;
    }

    return found_invalid_config ? DS_CONTROLLER_CONFIG_LOAD_INVALID : DS_CONTROLLER_CONFIG_LOAD_DEFAULT;
#endif
}
