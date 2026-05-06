# [Droidpad](https://github.com/umer0586/DroidPad) gamepad
Simple server that lets you use DroidPad as a game controller on Linux and Windows.

It currently supports:
- WebSocket transport
- Bluetooth RFCOMM transport
- Linux virtual controller output through `evdev`
- Windows output through ViGEmBus by default
- Windows `vJoy` backend
- Double-tap-to-hold button handling

# Requirements
- rust

## Windows
- [ViGEmBus](https://github.com/nefarius/ViGEmBus/releases) for the default backend
- [vJoy](https://github.com/BrunnerInnovation/vJoy/releases/tag/v2.2.2.0) for the `vjoy` backend

## Linux
- `libevdev`
- `BlueZ` (for Bluetooth transport)
- `libayatana-appindicator`
- `xdotool` (for tray functionality)
- `fontconfig`
- `gtk3`
- `wayland` / `libxkbcommon` (for UI)
- `dbus`

### Virtual Controller Permissions (udev)
To allow Droidpad to create a virtual controller without running as root, you need to install the provided udev rule:

```bash
sudo cp res/99-droidpad-gamepad.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules && sudo udevadm trigger
```

# Running
```bash
droidpad-gamepad
```

The server listens on port `1715` by default.

## Useful cli options
```bash
droidpad-gamepad --port 1715
droidpad-gamepad --double-tap-timing 200 --double-tap-postfix _dth
```

## Transports
WebSocket is the default transport:
```bash
droidpad-gamepad --transport ws
```

Bluetooth uses RFCOMM:
```bash
droidpad-gamepad --transport bluetooth
```

On Linux you can also choose the RFCOMM channel:
```bash
droidpad-gamepad --transport bluetooth --bt-channel 3
```

### Windows backends
ViGEmBus is the default backend.

To select the `vJoy` backend at runtime on Windows:
```bash
droidpad-gamepad --backend vjoy --vjoy-device 0
```

# Packages

## Flatpak

You can build and run Droidpad Gamepad as a Flatpak.

### Prerequisites
- `flatpak`
- `flatpak-builder`

### Building
```bash
flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak-builder --user --install --install-deps-from=flathub --force-clean build flatpak/io.github.tofixrs.droidpad-gamepad.yaml
```

If you are using the Nix devShell, you can simply run:
```bash
flatpak-build
```

### Running
```bash
flatpak run io.github.tofixrs.droidpad-gamepad
```

## Nix
### Flake
```nix
{
  inputs = {
    droidpad-gamepad = {
     url = "github:Tofixrs/droidpad-gamepad";
     inputs.nixpkgs.follows = "nixpkgs";
    }
  };
}
```
#### Cachix

```nix
# cachix
{
  nix.settings = {
    substituters = ["https://tofix-cache.cachix.org"];
    trusted-public-keys = ["tofix-cache.cachix.org-1:myU8xgZK0u4kkBPCBAlLH/8wCzw5Gn6OYpit6OsAhjU="];
  };
}
```

### NixOS Module
You can use the provided NixOS module to install the package and automatically set up the udev rules.

```nix
{ inputs, ... }: {
  imports = [ inputs.droidpad-gamepad.nixosModules.default ];
  services.droidpad-gamepad.enable = true;
}
```
