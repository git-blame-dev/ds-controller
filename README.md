# ds-controller

[![CI](https://github.com/git-blame-dev/ds-controller/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/git-blame-dev/ds-controller/actions/workflows/ci.yml)

Use a Nintendo DS or DS Lite as a wireless controller for Windows games.

The DS homebrew app reads the built-in buttons and sends compact UDP controller-state packets over Wi-Fi. The Windows receiver is a Rust CLI app that maps those packets to a ViGEm virtual Xbox 360 controller, so games see normal XInput input.

## Architecture

```text
Nintendo DS / DS Lite
  buttons -> UDP packets at 60 Hz
        |
        v
Windows receiver
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
- `cargo-xwin` for building the Windows receiver from Linux: `cargo install cargo-xwin --locked`
- devkitPro with devkitARM, libnds, and dswifi for building the DS sender
- Docker with `devkitpro/devkitarm:latest` if you prefer building the DS sender without installing devkitPro locally

## Configuration

Copy the example build config and set the Windows PC LAN address:

```sh
cp build.example.mk build.mk
```

Edit `build.mk`:

```make
PC_IP := 192.168.1.50
PC_PORT := 26760
```

`build.mk` is ignored by Git because it is local network configuration.

## Build

Build the DS ROM:

```sh
make nds
```

If devkitPro is not installed locally, build with Docker:

```sh
docker run --rm -v "$PWD":/work -w /work devkitpro/devkitarm:latest make nds
```

Build the Windows receiver:

```sh
make pc
```

Artifacts:

- `nds/build/ds-controller.nds`
- `pc/target/x86_64-pc-windows-msvc/release/ds-controller-pc.exe`

## Run

Start the Windows receiver first:

```sh
ds-controller-pc --bind 0.0.0.0:26760 --accept-first-sender
```

Then launch `nds/build/ds-controller.nds` on the DS.

The DS screen shows Wi-Fi connection progress. After connecting, the top screen turns off and the bottom screen stays off until touched. Touch wakes the status screen briefly; normal button input does not wake it.

## Development

Run all deterministic tests:

```sh
make test
```

Run Rust formatting, tests, linting, and the Windows GNU target check:

```sh
make pc-check
```

The DS host tests cover packet encoding, input mapping, and display wake policy. Hardware behavior such as Wi-Fi association and backlight control still requires a real DS or DS Lite.

## Limitations

- Buttons only; touchscreen input is used only to wake the status screen.
- DS / DS Lite Wi-Fi requires open or WEP-era 2.4 GHz networking.
- ViGEmBus is required for virtual Xbox 360 output on Windows.
