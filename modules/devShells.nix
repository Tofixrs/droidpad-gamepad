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
          flatpak-builder --user --install --install-deps-from=flathub --force-clean build flatpak/io.github.tofixrs.droidpad-gamepad.yaml
        '')
        appstream
      ];
      shellHook = ''
        export LD_LIBRARY_PATH="${runtimeLibraryPath}''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
      '';
    };
  };
}
