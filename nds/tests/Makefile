CC ?= gcc
CFLAGS ?= -std=c11 -Wall -Wextra -Werror -pedantic -DDS_CONTROLLER_HOST_TEST

PROTOCOL_SOURCES := protocol_test.c ../source/protocol.c ../source/input.c
PROTOCOL_TARGET := ../build/tests/protocol_test
DISPLAY_POLICY_SOURCES := display_policy_test.c ../source/display_policy.c
DISPLAY_POLICY_TARGET := ../build/tests/display_policy_test
TARGETS := $(PROTOCOL_TARGET) $(DISPLAY_POLICY_TARGET)

.PHONY: test clean

test: $(TARGETS)
	./$(PROTOCOL_TARGET)
	./$(DISPLAY_POLICY_TARGET)

$(PROTOCOL_TARGET): $(PROTOCOL_SOURCES)
	mkdir -p ../build/tests
	$(CC) $(CFLAGS) -I../source $(PROTOCOL_SOURCES) -o $(PROTOCOL_TARGET)

$(DISPLAY_POLICY_TARGET): $(DISPLAY_POLICY_SOURCES)
	mkdir -p ../build/tests
	$(CC) $(CFLAGS) -I../source $(DISPLAY_POLICY_SOURCES) -o $(DISPLAY_POLICY_TARGET)

clean:
	rm -f $(TARGETS)
