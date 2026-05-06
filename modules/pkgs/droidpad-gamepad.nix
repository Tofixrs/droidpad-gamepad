{
  craneLib,
  pkgs,
  self,
  ...
}: let
  runtimeLibs = with pkgs; [
    wayland
    libxkbcommon
    vulkan-loader
    libayatana-appindicator
  ];
  runtimeLibraryPath = pkgs.lib.makeLibraryPath runtimeLibs;
in
  craneLib.buildPackage {
    pname = "droidpad-gamepad";
    src = craneLib.cleanCargoSource ../..;
    version = "git-${toString (self.shortRev or self.dirtyShortRev or self.lastModified or "unknown")}";
    strictDeps = true;

    nativeBuildInputs = with pkgs; [
      pkg-config
    ];

    buildInputs = with pkgs; [
      dbus
      libevdev
      libxkbcommon
      libxcb
      wayland
      vulkan-loader
      libayatana-appindicator
      xdotool
      fontconfig
      glib
      gdk-pixbuf
      atk
      pango
      gtk3
    ];

    postFixup = ''
      wrapProgram "$out/bin/droidpad-gamepad" \
        --prefix LD_LIBRARY_PATH : "${runtimeLibraryPath}"

      install -Dm644 ${../../res/99-droidpad-gamepad.rules} $out/lib/udev/rules.d/99-droidpad-gamepad.rules
      install -Dm644 ${../../res/io.github.tofixrs.droidpad-gamepad.desktop} $out/share/applications/io.github.tofixrs.droidpad-gamepad.desktop
      install -Dm644 ${../../res/icon.png} $out/share/icons/hicolor/512x512/apps/io.github.tofixrs.droidpad-gamepad.png
      install -Dm644 ${../../res/io.github.tofixrs.droidpad-gamepad.metainfo.xml} $out/share/metainfo/io.github.tofixrs.droidpad-gamepad.metainfo.xml
    '';
  }
