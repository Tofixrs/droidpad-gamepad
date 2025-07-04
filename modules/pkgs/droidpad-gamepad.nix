{
  craneLib,
  pkgs,
  ...
}:
craneLib.buildPackage {
  pname = "droidpad-gamepad";
  src = craneLib.cleanCargoSource ../..;
  strictDeps = true;

  nativeBuildInputs = with pkgs; [
    pkg-config
  ];
  buildInputs = with pkgs; [
    libevdev
  ];
}
