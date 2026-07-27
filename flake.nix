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
          rustToolchain = p: p.rust-bin.stable.latest.default;
          craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rustToolchain;
          rust-src = pkgs.lib.cleanSourceWith {
            src = pkgs.lib.cleanSource ./.;
            filter = path: type: let
              relPath = pkgs.lib.removePrefix (toString ./. + "/") (toString path);
              isIgnoredDir = pkgs.lib.any (dir: pkgs.lib.hasPrefix dir relPath) [
                "target"
                ".git"
                ".direnv"
                ".flatpak-builder"
                "build"
                "flatpak"
                "third_party/zed/assets"
                "third_party/zed/docs"
                "third_party/zed/script"
                "third_party/zed/extensions"
              ];
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
              !isIgnoredDir && (isAsset || (craneLib.filterCargoSources path type));
          };
        };
      };
      imports = [
        ./modules
      ];
      systems = ["x86_64-linux"];
    };
}

