# [Droidpad](https://github.com/umer0586/DroidPad) gamepad
Simple websocket server that allows you to use droidpad as a game controller

# Requirements
- rust

## Windows
- [vJoy](https://github.com/BrunnerInnovation/vJoy/releases/tag/v2.2.2.0)

## Linux
- libevdev

# Packages

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
