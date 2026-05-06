_: {
  perSystem = {
    self',
    pkgs,
    lib,
    ...
  }: let
    runtimeLibs = [
      pkgs.wayland
      pkgs.libxkbcommon
      pkgs.vulkan-loader
      pkgs.libayatana-appindicator
    ];
    runtimeLibraryPath = lib.makeLibraryPath runtimeLibs;
  in {
    devShells.default = pkgs.mkShell {
      name = "shell";
      inputsFrom = [
        self'.packages.ui
      ];
      packages = with pkgs; [
        rust-analyzer
        pkg-config
        (writeShellScriptBin "flatpak-build" ''
          flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
          MANIFEST="flatpak/io.github.tofixrs.droidpad-gamepad.yaml"
          if [ "$1" = "cli" ]; then
            MANIFEST="flatpak/io.github.tofixrs.droidpad-gamepad-cli.yaml"
          fi
          flatpak-builder --user --install --install-deps-from=flathub --force-clean build "$MANIFEST"
        '')
        appstream
      ];
      shellHook = ''
        export LD_LIBRARY_PATH="${runtimeLibraryPath}''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
      '';
    };
  };
}
