# ds-controller

[![CI](https://github.com/git-blame-dev/ds-controller/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/git-blame-dev/ds-controller/actions/workflows/ci.yml)

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
- Docker with `devkitpro/devkitarm:latest` if you prefer building the DS sender without installing devkitPro locally

## Configuration

The DS ROM can read a `ds-controller.ini` file from these flashcart paths:

- `ds-controller.ini`
- `/ds-controller.ini`
- `/ds-controller/ds-controller.ini`

```ini
pc_ip=192.168.1.50
pc_port=26760
```

Use `nds/ds-controller.example.ini` as a starting point. If no config file is found, the ROM uses the build-time defaults.

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

```sh
docker run --rm -v "$PWD":/work -w /work devkitpro/devkitarm:latest make nds
```

Build the Windows PC GUI app:

```sh
make pc
```

Artifacts:

- `nds/build/ds-controller.nds`
- Portable Windows app files under `pc/target/x86_64-pc-windows-msvc/release/`:
  - `ds-controller.exe`
  - `WebView2Loader.dll`

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

Run Rust formatting, receiver tests, receiver linting, the Windows GNU receiver check, and frontend checks:

```sh
make pc-check
```

The Tauri app has additional platform prerequisites. If Linux Tauri checks fail because system GUI libraries such as `pkg-config`, `dbus`, or WebKitGTK are missing, install the Tauri Linux prerequisites or validate the GUI build on Windows.

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
