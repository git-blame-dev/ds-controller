# 🎮 DS Controller

[![CI](https://github.com/git-blame-dev/ds-controller/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/git-blame-dev/ds-controller/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/git-blame-dev/ds-controller?label=release)](https://github.com/git-blame-dev/ds-controller/releases/latest)
[![Windows app](https://img.shields.io/badge/Windows%20app-in%20release%20zip-0078D4)](https://github.com/git-blame-dev/ds-controller/releases/latest)
[![Nintendo DS ROM](https://img.shields.io/badge/DS%20ROM-in%20release%20zip-blue)](https://github.com/git-blame-dev/ds-controller/releases/latest)

Use a Nintendo DS or DS Lite as a Wi-Fi controller for PC games through a virtual game controller on Windows or Ubuntu.

![DS Controller receiver dashboard beside a Nintendo DS controlling a game over Wi-Fi](demo.webp)

## 🔎 Overview

DS Controller turns original Nintendo DS-family hardware into a wireless PC game controller. The DS homebrew app reads the handheld's buttons and sends compact UDP controller-state packets over Wi-Fi; the desktop receiver maps those packets to a ViGEm virtual Xbox 360 controller on Windows or a `uinput` virtual gamepad on Ubuntu.

The project includes both sides of the system: a Nintendo DS ROM for the sender and a dark desktop receiver dashboard for Windows and Ubuntu.

## ✨ Features

- Use Nintendo DS / DS Lite buttons as gamepad input over local Wi-Fi.
- Output through ViGEm on Windows or the Linux `uinput` interface on Ubuntu.
- Configure the receiver UDP port from the PC dashboard and restart the receiver without relaunching the app.
- View receiver status, virtual-controller status, packet debugging, and logs in one desktop window.
- Configure the DS target PC from `ds-controller.ini`, with build-time defaults available for fixed setups.
- Keep the DS display mostly off during play; touch wakes the status screen briefly.

## 🛠️ Tech Stack

- **Nintendo DS ROM:** C homebrew built with devkitPro, devkitARM, libnds, and dswifi.
- **PC receiver backend:** Rust, UDP socket handling, ViGEm on Windows, and `evdev`/`uinput` on Ubuntu.
- **Desktop UI:** Tauri 2, React 19, TypeScript, Vite, and pnpm.
- **Build / release tooling:** Make targets for deterministic tests, DS ROM builds, Ubuntu Debian packages, and Linux-first Windows cross-builds.
- **CI / artifacts:** GitHub Actions release workflow with Windows app files, the NDS ROM, and example configuration.

## 🧠 Engineering Highlights

- Splits the system at a small UDP packet boundary: the DS only sends button state, while the PC owns filtering, timeout behavior, and virtual controller output.
- Uses ViGEm on Windows and `uinput` on Ubuntu to expose a standard virtual game controller instead of requiring per-game keyboard mapping.
- Supports both runtime DS configuration via `ds-controller.ini` and optional build-time defaults through `build.mk` for flashcart workflows.
- Keeps hardware-specific behavior explicit: Wi-Fi association, backlight control, and virtual-controller runtime behavior still require real DS and target-platform validation.
- Uses a Linux-first workflow for deterministic checks, native Ubuntu packages, and cross-built Windows artifacts.

## 🏗️ Architecture

```text
Nintendo DS / DS Lite
  buttons -> UDP packets at 60 Hz
        |
        v
Windows or Ubuntu PC receiver
  parse -> filter -> timeout -> ViGEm or uinput virtual controller
        |
        v
PC game using platform controller input
```

The DS sender is intentionally small: it connects to a DS-compatible Wi-Fi profile, reads button input, and sends controller packets to the configured PC LAN IP and UDP port.

The desktop app receives packets, updates the dashboard, handles receiver lifecycle controls, and writes controller state through ViGEm on Windows or `uinput` on Ubuntu. Game compatibility still depends on the platform virtual-controller stack and the target game's controller support.

Key directories:

- `nds/` - Nintendo DS sender, homebrew build files, DS-side config example, and host-side tests.
- `pc/receiver/` - Rust receiver logic for UDP packet handling and controller output boundaries.
- `pc/app/` - Tauri/React desktop dashboard for receiver status, controls, and logs.

## 🚀 Getting Started

### Prerequisites

Runtime:

- Nintendo DS or DS Lite.
- Flashcart or compatible homebrew loader.
- Windows PC or Ubuntu 24.04 x86_64 PC on the same LAN as the DS.
- [ViGEmBus](https://github.com/nefarius/ViGEmBus/releases) installed when using Windows.
- DS-compatible 2.4 GHz Wi-Fi network.

Build tooling:

- Rust toolchain.
- Node.js and pnpm for the Tauri GUI frontend.
- Tauri 2 native dependencies for the target desktop platform.
- `cargo-xwin` or native Windows tooling for Windows Rust validation.
- [devkitPro](https://devkitpro.org/wiki/Getting_Started) with devkitARM, libnds, and dswifi for building the DS sender.
- Docker for building the DS sender with the pinned devkitARM image used by CI.

### DS Wi-Fi setup

A DS-compatible Wi-Fi network means an old-style 2.4 GHz `802.11b` network using open or WEP security. DS / DS Lite cannot connect to WPA, WPA2, WPA3, or 5 GHz networks.

Configure the Wi-Fi profile first from a Nintendo WFC-compatible DS game, such as Mario Kart DS or a Generation 4 Pokemon game, then launch the DS sender. Use a dedicated isolated network if you use open or WEP.

### Configure the DS target PC

The DS ROM can read a `ds-controller.ini` file from these flashcart paths:

- `ds-controller.ini`
- `/ds-controller.ini`
- `/ds-controller/ds-controller.ini`

```ini
pc_ip=192.168.1.50
pc_port=26760
```

Use [`nds/ds-controller.ini`](nds/ds-controller.ini) as a starting point. Set `pc_ip` to the receiver PC's LAN IP address. Leave `pc_port` as `26760` unless that port is already in use or you changed the PC receiver port. If no config file is found, the ROM uses the build-time defaults.

Optional build-time configuration:

```sh
cp build.example.mk build.mk
```

Copy [`build.example.mk`](build.example.mk), then edit `build.mk`:

```make
PC_IP := 192.168.1.50
PC_PORT := 26760
```

`build.mk` is ignored by Git because it is local network configuration. It is optional when using `ds-controller.ini`.

### Build locally

On Ubuntu 24.04, install the native desktop build dependencies:

```sh
sudo apt install clang libayatana-appindicator3-dev libgtk-3-dev librsvg2-dev \
  libwebkit2gtk-4.1-dev libxdo-dev lld llvm patchelf pkg-config
```

Install the PC app tooling once from the repo root:

```sh
pnpm install --frozen-lockfile
```

Build the DS ROM:

```sh
make nds
```

The default `make nds` target uses Docker with the same pinned devkitARM image as CI when devkitPro is not installed locally.

Build the Ubuntu `.deb`:

```sh
make linux-dist
```

The package lands at `dist/linux/ds-controller-linux-amd64.deb`. Install it with:

```sh
sudo apt install ./dist/linux/ds-controller-linux-amd64.deb
```

Installation adds a udev rule that grants the active desktop user access to `/dev/uinput`. DS Controller itself runs as the normal user; do not run the GUI with `sudo`.

Cross-build the Windows PC GUI app from Linux:

```sh
make pc
```

To stage all three local release artifacts:

```sh
make dist
```

The staged Ubuntu package lands in `dist/linux/`, Windows app files land in `dist/pc/`, and DS files land in `dist/nds/`.

`make pc` requires LLVM tools, `cargo-xwin`, and the Windows MSVC Rust target:

```sh
rustup target add x86_64-pc-windows-msvc
cargo install cargo-xwin
```

Install LLVM tools with one of:

```sh
# Ubuntu / WSL
sudo apt install clang lld llvm

# CachyOS / Arch
sudo pacman -S --needed clang lld llvm
```

### Run

Copy `dist/pc/ds-controller.exe` and `dist/pc/WebView2Loader.dll` to the same folder on the Windows PC, then run `ds-controller.exe`. The receiver starts automatically when **Start receiver when app opens** is enabled. You can change the UDP port, use **Apply & Restart**, and view receiver logs in the app.

On Ubuntu, install the `.deb` and launch **DS Controller** from the application menu or run `ds-controller`. The package configures `/dev/uinput` access, and the app creates a `DS Controller Virtual Gamepad` while the receiver is running.

If UFW is active, allow the configured UDP port from the DS network. Replace the subnet and port if your LAN differs:

```sh
sudo ufw allow from 192.168.1.0/24 to any port 26760 proto udp comment 'DS Controller'
```

The package does not change firewall policy automatically.

The udev rule persists across normal reboots, so the package only needs to be installed once. After uninstalling DS Controller, reboot once to clear any `/dev/uinput` access retained by the current desktop session.

Then launch `dist/nds/ds-controller.nds` on the DS.

The DS screen shows Wi-Fi connection progress. After connecting, the top screen turns off and the bottom screen stays off until touched. Touch wakes the status screen briefly; normal button input does not wake it.

## ✅ Testing

Run all deterministic tests:

```sh
make test
```

For a manual PC GUI smoke check, run the app in development mode:

```sh
make app-dev
```

Lean local workflow: `make test` validates the code; `make linux-verify` builds and inspects the Ubuntu package; `make dist` stages all release artifacts.

CI runs deterministic tests, cross-builds the Windows app, builds the NDS ROM, and stages artifacts under `dist/pc` and `dist/nds`.

The DS host tests cover packet encoding, input mapping, and display wake policy. Hardware behavior such as Wi-Fi association, backlight control, WebView2 startup, ViGEmBus integration, Linux game detection, firewall prompts, and real controller output still requires manual platform and DS validation.

## 📦 Releases / Artifacts

[GitHub Releases](https://github.com/git-blame-dev/ds-controller/releases) publish a complete zip containing the Windows app files, NDS ROM, and `ds-controller.ini`.

For local builds and manual testing, artifacts are staged at:

- DS files: `dist/nds/`
- Ubuntu package: `dist/linux/ds-controller-linux-amd64.deb`
- Windows executable: `dist/pc/ds-controller.exe`
- WebView2 loader DLL: `dist/pc/WebView2Loader.dll`

When testing manually on Windows, keep `ds-controller.exe` and `WebView2Loader.dll` together. On Ubuntu, install the `.deb` so its package-owned udev rule is applied. CI uploads the staged Windows and NDS artifact directories.

## ⚠️ Limitations

- Windows game compatibility is limited to games that accept Xbox 360 / XInput controllers through the virtual controller layer.
- Controller input is button-only; touchscreen input is used only to wake the status screen.
- DS / DS Lite Wi-Fi requires open or WEP-era 2.4 GHz networking.
- Do not connect open or WEP Wi-Fi to your main network; use an isolated network segment for DS testing.
- ViGEmBus is required for virtual Xbox 360 output on Windows.
- Initial Linux package support is limited to Ubuntu 24.04 x86_64.
- End-to-end behavior still needs validation on an actual DS or DS Lite and the target Windows or Ubuntu PC.

## 🧯 Troubleshooting

- **ViGEm error:** install ViGEmBus, then restart DS Controller.
- **Linux virtual controller error:** install the `.deb`, confirm `test -w /dev/uinput`, then restart DS Controller.
- **Port already in use:** choose a different port in the app and click **Apply & Restart**.
- **No packets received:** confirm the DS is using the receiver PC's LAN IP address and the same UDP port shown in the app.
- **Sender config changes do not apply:** if you use build-time defaults, rebuild the ROM after changing `build.mk`; if you use `ds-controller.ini`, confirm the file is on one of the supported flashcart paths.
- **UDP traffic is blocked:** add a subnet-scoped UFW rule for the selected UDP port, as shown in the Ubuntu run instructions.
- **DS Wi-Fi issue:** confirm the network is 2.4 GHz `802.11b` with open or WEP security, and configure the Wi-Fi profile from a Nintendo WFC-compatible DS game before launching the sender.
- **Game does not respond:** on Windows, confirm ViGEmBus is installed and the game accepts Xbox 360 / XInput controllers; on Ubuntu, confirm the virtual gamepad appears in the system input inventory and the game accepts Linux gamepads.
