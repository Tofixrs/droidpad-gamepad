{
  description = "Droidpad server to emulate a gamepad on linux";

  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = inputs @ {flake-parts, ...}:
    flake-parts.lib.mkFlake {inherit inputs;} {
      perSystem = {system, ...}: {
        _module.args = rec {
          pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [
              (import inputs.rust-overlay)
            ];
          };
          rustToolchain = pkgs.rust-bin.selectLatestNightlyWith (toolchain:
            toolchain.default.override {
              extensions = ["rustc-codegen-cranelift-preview"];
            });
          craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rustToolchain;
        };
      };
      imports = [
        ./modules
      ];
      systems = ["x86_64-linux"];
    };
}
