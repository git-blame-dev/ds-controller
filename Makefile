.SUFFIXES:

WINDOWS_TARGET := x86_64-pc-windows-msvc
DEVKITARM_IMAGE ?= devkitpro/devkitarm:latest
DOCKER_USER := $(shell id -u):$(shell id -g)
PC_PORT ?= 26760

-include build.mk

.DEFAULT_GOAL := help

.PHONY: help all nds nds-local pc app-dev frontend-check test pc-check clean check-pc-ip

help:
	@printf '%s\n' 'Targets:'
	@printf '%s\n' '  make nds       Build the Nintendo DS ROM in Docker using PC_IP/PC_PORT from build.mk'
	@printf '%s\n' '  make nds-local Build the Nintendo DS ROM with local devkitPro'
	@printf '%s\n' '  make pc        Build the Windows Tauri GUI app bundle'
	@printf '%s\n' '  make app-dev   Run the Tauri GUI app in development mode'
	@printf '%s\n' '  make test      Run DS host tests, receiver Rust tests, and frontend build'
	@printf '%s\n' '  make pc-check  Run receiver Rust checks and frontend checks'
	@printf '%s\n' '  make all       Build both release artifacts'

all: nds pc

nds: check-pc-ip
	docker run --rm --user $(DOCKER_USER) -e HOME=/tmp -v "$(CURDIR)":/workspace -w /workspace \
	$(DEVKITARM_IMAGE) \
	$(MAKE) nds-local PC_IP="$(PC_IP)" PC_PORT="$(PC_PORT)"
	@printf 'NDS ROM: %s\n' "$(CURDIR)/nds/build/ds-controller.nds"

nds-local: check-pc-ip
	$(MAKE) -C nds clean
	$(MAKE) -C nds PC_IP="$(PC_IP)" PC_PORT="$(PC_PORT)"
	@printf 'NDS ROM: %s\n' "$(CURDIR)/nds/build/ds-controller.nds"

pc:
	pnpm --dir pc/app tauri build --target $(WINDOWS_TARGET)
	@printf 'Windows GUI app bundle: %s\n' "$(CURDIR)/target/$(WINDOWS_TARGET)/release/bundle"

app-dev:
	pnpm --dir pc/app tauri dev

frontend-check:
	pnpm --dir pc/app lint
	pnpm --dir pc/app test
	pnpm --dir pc/app build

test:
	$(MAKE) -C nds test
	cargo test --manifest-path pc/Cargo.toml
	pnpm --dir pc/app build

pc-check:
	cargo fmt --all --check
	cargo test --manifest-path pc/Cargo.toml
	cargo clippy --manifest-path pc/Cargo.toml -- -D warnings
	cargo check --manifest-path pc/Cargo.toml --target x86_64-pc-windows-gnu
	$(MAKE) frontend-check

clean:
	$(MAKE) -C nds clean
	cargo clean

check-pc-ip:
	@if [ -z "$(PC_IP)" ]; then \
		printf '%s\n' 'PC_IP is required for NDS builds.'; \
		printf '%s\n' 'Copy build.example.mk to ignored build.mk, then set PC_IP=<windows-pc-lan-ip>.'; \
		exit 1; \
	fi
