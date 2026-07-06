_: {
  perSystem = {
    pkgs,
    craneLib,
    rust-src,
    ...
  }: let
    cargoToml = builtins.fromTOML (builtins.readFile ../../Cargo.toml);
    version = cargoToml.package.version;
    commonArgs = {
      inherit version;
      src = rust-src;
      strictDeps = true;
      nativeBuildInputs = with pkgs; [
        pkg-config
        makeWrapper
      ];
      buildInputs = with pkgs; [
        dbus
        libevdev
      ];
    };

    cliArgs =
      commonArgs
      // {
        pname = "droidpad-gamepad-cli";
        cargoExtraArgs = "--no-default-features --features ws,bluetooth";
      };

    uiArgs =
      commonArgs
      // {
        pname = "droidpad-gamepad-ui";
        cargoExtraArgs = "--features ui,ws,bluetooth";
        buildInputs =
          commonArgs.buildInputs
          ++ (with pkgs; [
            libxkbcommon
            libxcb
            libayatana-appindicator
            xdotool
            fontconfig
            glib
            gdk-pixbuf
            atk
            pango
            wayland
            vulkan-loader
            gtk3
            libx11
            libxcursor
            libxi
            libxrandr
            libxrender
            libxcomposite
            libxdamage
            libxext
            libxfixes
            libxinerama
            libGL
            freetype
            curl
            openssl
            alsa-lib
          ]);
      };

    cargoCliArtifacts = craneLib.buildDepsOnly cliArgs;
    cargoUiArtifacts = craneLib.buildDepsOnly uiArgs;

    runtimeLibs = with pkgs; [
      wayland
      libxkbcommon
      vulkan-loader
      libayatana-appindicator
    ];
    runtimeLibraryPath = pkgs.lib.makeLibraryPath runtimeLibs;

    postFixup = {
      binName,
      runtimeLibPath ? "",
    }: ''
      ${pkgs.lib.optionalString (runtimeLibPath != "") ''
        wrapProgram "$out/bin/${binName}" \
          --prefix LD_LIBRARY_PATH : "${runtimeLibPath}"
      ''}

      install -Dm644 ${../../res/99-droidpad-gamepad.rules} $out/lib/udev/rules.d/99-droidpad-gamepad.rules
      install -Dm644 ${../../res/io.github.tofixrs.droidpad-gamepad.desktop} $out/share/applications/io.github.tofixrs.droidpad-gamepad.desktop
      install -Dm644 ${../../res/icon.png} $out/share/icons/hicolor/512x512/apps/io.github.tofixrs.droidpad-gamepad.png
      install -Dm644 ${../../res/io.github.tofixrs.droidpad-gamepad.metainfo.xml} $out/share/metainfo/io.github.tofixrs.droidpad-gamepad.metainfo.xml
    '';
  in {
    packages = rec {
      cli = craneLib.buildPackage (cliArgs
        // {
          cargoArtifacts = cargoCliArtifacts;
          postFixup = postFixup {binName = "droidpad-gamepad";};
        });
      ui = craneLib.buildPackage (uiArgs
        // {
          cargoArtifacts = cargoUiArtifacts;
          postFixup = postFixup {
            binName = "droidpad-gamepad";
            runtimeLibPath = runtimeLibraryPath;
          };
        });
      default = ui;
    };
  };
}
