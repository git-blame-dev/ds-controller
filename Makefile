.SUFFIXES:

WINDOWS_TARGET := x86_64-pc-windows-msvc
DEVKITARM_IMAGE ?= devkitpro/devkitarm:20260221@sha256:4debd5b33cf4361a557b6bf3be5ff823804868125ce1429912f1a4e773e7ac5d
DOCKER_USER := $(shell id -u):$(shell id -g)
PC_PORT ?= 26760

-include build.mk

PC_IP ?= 192.0.2.1

.DEFAULT_GOAL := help

.PHONY: help all nds nds-local pc app-dev test clean

help:
	@printf '%s\n' 'Targets:'
	@printf '%s\n' '  make nds       Build the Nintendo DS ROM in Docker; PC_IP/PC_PORT from build.mk are optional defaults'
	@printf '%s\n' '  make nds-local Build the Nintendo DS ROM with local devkitPro'
	@printf '%s\n' '  make pc        Cross-build the portable Windows Tauri GUI app from Linux'
	@printf '%s\n' '  make app-dev   Run the Tauri GUI app in development mode'
	@printf '%s\n' '  make test      Run DS host tests, receiver Rust tests, and frontend build'
	@printf '%s\n' '  make all       Build both release artifacts'

all: nds pc

nds:
	docker run --rm --user $(DOCKER_USER) -e HOME=/tmp -v "$(CURDIR)":/workspace -w /workspace \
	$(DEVKITARM_IMAGE) \
	$(MAKE) nds-local PC_IP="$(PC_IP)" PC_PORT="$(PC_PORT)"
	@printf 'NDS ROM: %s\n' "$(CURDIR)/nds/build/ds-controller.nds"

nds-local:
	$(MAKE) -C nds clean
	$(MAKE) -C nds PC_IP="$(PC_IP)" PC_PORT="$(PC_PORT)"
	@printf 'NDS ROM: %s\n' "$(CURDIR)/nds/build/ds-controller.nds"

pc:
	@command -v cargo-xwin >/dev/null || { printf '%s\n' 'Missing cargo-xwin. Install with: cargo install cargo-xwin' >&2; exit 1; }
	@command -v llvm-rc >/dev/null || { printf '%s\n' 'Missing llvm-rc. Install LLVM tools before running make pc.' >&2; exit 1; }
	@command -v lld-link >/dev/null || { printf '%s\n' 'Missing lld-link. Install lld before running make pc.' >&2; exit 1; }
	pnpm --dir pc/app tauri build --runner "$(CURDIR)/scripts/cargo-xwin-runner.sh" --target $(WINDOWS_TARGET) --no-bundle
	@printf 'Windows GUI app: %s\n' "$(CURDIR)/pc/target/$(WINDOWS_TARGET)/release/ds-controller.exe"

app-dev:
	pnpm --dir pc/app tauri dev

test:
	$(MAKE) -C nds test
	cargo test --manifest-path pc/Cargo.toml
	pnpm --dir pc/app build

clean:
	$(MAKE) -C nds clean
	cargo clean
