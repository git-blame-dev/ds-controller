#!/bin/sh
set -eu

if [ -d /run/udev ] && command -v udevadm >/dev/null 2>&1; then
    udevadm control --reload-rules || true
    udevadm trigger --subsystem-match=misc --sysname-match=uinput || true
fi

exit 0
