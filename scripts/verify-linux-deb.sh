#!/bin/sh
set -eu

package_path=${1:?"usage: verify-linux-deb.sh PACKAGE.deb"}
expected_version=${2:?"usage: verify-linux-deb.sh PACKAGE.deb EXPECTED_VERSION"}
expected_maintainer='git-blame-dev <251953086+git-blame-dev@users.noreply.github.com>'
expected_homepage='https://github.com/git-blame-dev/ds-controller'

if [ ! -f "$package_path" ]; then
    printf 'Debian package does not exist: %s\n' "$package_path" >&2
    exit 1
fi

if [ "$(dpkg-deb --field "$package_path" Package)" != "ds-controller" ]; then
    printf '%s\n' 'Unexpected Debian package name' >&2
    exit 1
fi

if [ "$(dpkg-deb --field "$package_path" Architecture)" != "amd64" ]; then
    printf '%s\n' 'Unexpected Debian package architecture' >&2
    exit 1
fi

if [ "$(dpkg-deb --field "$package_path" Version)" != "$expected_version" ]; then
    printf 'Unexpected Debian package version; expected %s\n' "$expected_version" >&2
    exit 1
fi

if [ "$(dpkg-deb --field "$package_path" Maintainer)" != "$expected_maintainer" ]; then
    printf 'Unexpected Debian package maintainer; expected %s\n' "$expected_maintainer" >&2
    exit 1
fi

if [ "$(dpkg-deb --field "$package_path" Homepage)" != "$expected_homepage" ]; then
    printf 'Unexpected Debian package homepage; expected %s\n' "$expected_homepage" >&2
    exit 1
fi

depends=$(dpkg-deb --field "$package_path" Depends)
for dependency in libwebkit2gtk-4.1-0 libgtk-3-0; do
    if ! printf '%s\n' "$depends" | tr ',' '\n' \
        | grep -Eq "^[[:space:]]*${dependency}([[:space:](]|$)"; then
        printf 'Debian package is missing dependency: %s\n' "$dependency" >&2
        exit 1
    fi
done

work_dir=$(mktemp -d)
trap 'rm -rf "$work_dir"' EXIT HUP INT TERM

dpkg-deb --extract "$package_path" "$work_dir/payload"
dpkg-deb --control "$package_path" "$work_dir/control"

test -x "$work_dir/payload/usr/bin/ds-controller"
test -f "$work_dir/payload/usr/share/applications/DS Controller.desktop"
test -f "$work_dir/payload/usr/lib/udev/rules.d/70-ds-controller-uinput.rules"
test -x "$work_dir/control/postinst"
test -x "$work_dir/control/postrm"

cmp "$work_dir/control/postinst" "$work_dir/control/postrm"
cmp "$work_dir/control/postinst" "pc/app/src-tauri/linux/reload-uinput-rules.sh"
cmp "$work_dir/payload/usr/lib/udev/rules.d/70-ds-controller-uinput.rules" \
    "pc/app/src-tauri/linux/70-ds-controller-uinput.rules"

printf 'Verified Debian package: %s\n' "$package_path"
