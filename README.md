# ds-controller

[![CI](https://github.com/git-blame-dev/ds-controller/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/git-blame-dev/ds-controller/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/git-blame-dev/ds-controller?label=release)](https://github.com/git-blame-dev/ds-controller/releases/latest)

Use a Nintendo DS or DS Lite as a wireless controller for Windows games.

The DS homebrew app reads the built-in buttons and sends compact UDP controller-state packets over Wi-Fi. The Windows PC app is a dark Tauri desktop GUI that maps those packets to a ViGEm virtual Xbox 360 controller, so games see normal XInput input.

## PC app

The PC app is a portable single-window dark dashboard for receiver status, ViGEm status, port configuration, start/stop controls, packet debugging, and logs.

## Architecture

```text
Nintendo DS / DS Lite
  buttons -> UDP packets at 60 Hz
        |
        v
Windows PC app
  parse -> filter -> timeout -> Xbox 360 output
        |
        v
PC game
```

## Requirements

### Runtime

- Nintendo DS or DS Lite
- Flashcart or compatible homebrew loader
- Windows PC on the same LAN as the DS
- ViGEmBus installed on Windows
- DS-compatible 2.4 GHz Wi-Fi network

A DS-compatible Wi-Fi network means an old-style 2.4 GHz `802.11b` network using open or WEP security. DS / DS Lite cannot connect to WPA, WPA2, WPA3, or 5 GHz networks.

Configure the Wi-Fi profile first from a Nintendo WFC-compatible DS game, such as Mario Kart DS or a Generation 4 Pokemon game, then launch the DS sender. Use a dedicated isolated network if you use open or WEP.

### Build

- Rust toolchain
- Node.js and pnpm for the Tauri GUI frontend
- Tauri 2 system prerequisites for Windows app builds
- `cargo-xwin` or native Windows tooling for Windows Rust validation
- devkitPro with devkitARM, libnds, and dswifi for building the DS sender
- Docker for building the DS sender with the pinned devkitARM image used by CI

## Configuration

The DS ROM can read a `ds-controller.ini` file from these flashcart paths:

- `ds-controller.ini`
- `/ds-controller.ini`
- `/ds-controller/ds-controller.ini`

```ini
pc_ip=192.168.1.50
pc_port=26760
```

Use `nds/ds-controller.ini` as a starting point. Set `pc_ip` to the Windows receiver PC's LAN IP address. Leave `pc_port` as `26760` unless that port is already in use or you changed the PC receiver port. If no config file is found, the ROM uses the build-time defaults.

Copy the example build config and set the Windows PC LAN address:

```sh
cp build.example.mk build.mk
```

Edit `build.mk`:

```make
PC_IP := 192.168.1.50
PC_PORT := 26760
```

`build.mk` is ignored by Git because it is local network configuration. It is optional when using `ds-controller.ini`.

## Build

Build the DS ROM:

```sh
make nds
```

If devkitPro is not installed locally, build with Docker:

The default `make nds` target uses Docker with the same pinned devkitARM image as CI.

Cross-build the Windows PC GUI app from Linux:

```sh
make pc
```

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

Artifacts:

- `nds/build/ds-controller.nds`
- Windows executable: `pc/target/x86_64-pc-windows-msvc/release/ds-controller.exe`
- WebView2 loader DLL for manual Windows testing: `pc/target/x86_64-pc-windows-msvc/release/build/webview2-com-sys-*/out/x64/WebView2Loader.dll`

When testing manually on Windows, copy `ds-controller.exe` and `WebView2Loader.dll` into the same folder. CI runs the same Linux-first `make test` / `make pc` workflow and stages both files in the `ds-controller-pc-app` artifact.

GitHub Releases publish one complete zip containing the Windows app files, NDS ROM, and `ds-controller.ini`.

## Run

Copy `ds-controller.exe` and `WebView2Loader.dll` to the same folder on the Windows PC, then run `ds-controller.exe`. The receiver starts automatically when **Start receiver when app opens** is enabled. You can change the UDP port, use **Apply & Restart**, and view receiver logs in the app.

Then launch `nds/build/ds-controller.nds` on the DS.

The DS screen shows Wi-Fi connection progress. After connecting, the top screen turns off and the bottom screen stays off until touched. Touch wakes the status screen briefly; normal button input does not wake it.

## Development

Run all deterministic tests:

```sh
make test
```

Run the PC GUI in development mode:

```sh
make app-dev
```

Lean local workflow: `make test` validates the code; `make pc` produces the Windows executable.

The DS host tests cover packet encoding, input mapping, and display wake policy. Hardware behavior such as Wi-Fi association and backlight control still requires a real DS or DS Lite.

## Limitations

- Buttons only; touchscreen input is used only to wake the status screen.
- DS / DS Lite Wi-Fi requires open or WEP-era 2.4 GHz networking.
- ViGEmBus is required for virtual Xbox 360 output on Windows.

## Troubleshooting

- **ViGEm error:** install ViGEmBus, then restart DS Controller.
- **Port already in use:** choose a different port in the app and click **Apply & Restart**.
- **No packets received:** confirm the DS ROM was built with the PC LAN IP and the same UDP port shown in the app.
- **Firewall prompt:** allow DS Controller to receive UDP traffic on the selected port.
- **DS Wi-Fi issue:** DS / DS Lite requires open or WEP-era 2.4 GHz Wi-Fi.
