{
  description = "Droidpad server to emulate a gamepad on linux";

  inputs = {
    self.submodules = true;
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
          rustToolchain = p:
            p.rust-bin.selectLatestNightlyWith (toolchain:
              toolchain.default.override {
                extensions = ["rustc-codegen-cranelift-preview"];
              });
          craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rustToolchain;
          rust-src = pkgs.lib.cleanSourceWith {
            src = ./.;
            filter = path: type: let
              isAsset = pkgs.lib.any (suffix: pkgs.lib.hasSuffix suffix path) [
                ".md"
                ".png"
                ".ttf"
                ".scm"
                ".json"
                ".svg"
                ".wgsl"
              ];
            in
              isAsset || (craneLib.filterCargoSources path type);
          };
        };
      };
      imports = [
        ./modules
      ];
      systems = ["x86_64-linux"];
    };
}
