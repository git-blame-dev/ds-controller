CC ?= gcc
CFLAGS ?= -std=c11 -Wall -Wextra -Werror -pedantic -DDS_CONTROLLER_HOST_TEST

SOURCES := protocol_test.c ../source/protocol.c ../source/input.c
TARGET := ../build/tests/protocol_test

.PHONY: test clean

test: $(TARGET)
	./$(TARGET)

$(TARGET): $(SOURCES)
	mkdir -p ../build/tests
	$(CC) $(CFLAGS) -I../source $(SOURCES) -o $(TARGET)

clean:
	rm -f $(TARGET)
