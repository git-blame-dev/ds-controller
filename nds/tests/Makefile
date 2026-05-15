CC ?= gcc
CFLAGS ?= -std=c11 -Wall -Wextra -Werror -pedantic -DDS_CONTROLLER_HOST_TEST

PROTOCOL_SOURCES := protocol_test.c ../source/protocol.c ../source/input.c
PROTOCOL_TARGET := ../build/tests/protocol_test
DISPLAY_POLICY_SOURCES := display_policy_test.c ../source/display_policy.c
DISPLAY_POLICY_TARGET := ../build/tests/display_policy_test
RUNTIME_CONFIG_SOURCES := runtime_config_test.c ../source/runtime_config.c
RUNTIME_CONFIG_TARGET := ../build/tests/runtime_config_test
TARGETS := $(PROTOCOL_TARGET) $(DISPLAY_POLICY_TARGET) $(RUNTIME_CONFIG_TARGET)

.PHONY: test clean

test: $(TARGETS)
	./$(PROTOCOL_TARGET)
	./$(DISPLAY_POLICY_TARGET)
	./$(RUNTIME_CONFIG_TARGET)

$(PROTOCOL_TARGET): $(PROTOCOL_SOURCES)
	mkdir -p ../build/tests
	$(CC) $(CFLAGS) -I../source $(PROTOCOL_SOURCES) -o $(PROTOCOL_TARGET)

$(DISPLAY_POLICY_TARGET): $(DISPLAY_POLICY_SOURCES)
	mkdir -p ../build/tests
	$(CC) $(CFLAGS) -I../source $(DISPLAY_POLICY_SOURCES) -o $(DISPLAY_POLICY_TARGET)

$(RUNTIME_CONFIG_TARGET): $(RUNTIME_CONFIG_SOURCES)
	mkdir -p ../build/tests
	$(CC) $(CFLAGS) -DDS_CONTROLLER_PC_IP='"192.0.2.1"' -DDS_CONTROLLER_PC_PORT=26760 -I../source $(RUNTIME_CONFIG_SOURCES) -o $(RUNTIME_CONFIG_TARGET)

clean:
	rm -f $(TARGETS)
