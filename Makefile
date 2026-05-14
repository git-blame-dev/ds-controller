.SUFFIXES:

WINDOWS_TARGET := x86_64-pc-windows-msvc
DEVKITARM_IMAGE ?= devkitpro/devkitarm:latest
DOCKER_USER := $(shell id -u):$(shell id -g)
PC_PORT ?= 26760

-include build.mk

PC_IP ?= 192.0.2.1

.DEFAULT_GOAL := help

.PHONY: help all nds nds-local pc app-dev frontend-check test pc-check clean

help:
	@printf '%s\n' 'Targets:'
	@printf '%s\n' '  make nds       Build the Nintendo DS ROM in Docker; PC_IP/PC_PORT from build.mk are optional defaults'
	@printf '%s\n' '  make nds-local Build the Nintendo DS ROM with local devkitPro'
	@printf '%s\n' '  make pc        Build the portable Windows Tauri GUI app'
	@printf '%s\n' '  make app-dev   Run the Tauri GUI app in development mode'
	@printf '%s\n' '  make test      Run DS host tests, receiver Rust tests, and frontend build'
	@printf '%s\n' '  make pc-check  Run receiver Rust checks and frontend checks'
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
	pnpm --dir pc/app tauri build --target $(WINDOWS_TARGET) --no-bundle
	@printf 'Windows GUI app: %s\n' "$(CURDIR)/pc/target/$(WINDOWS_TARGET)/release/ds-controller.exe"

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
