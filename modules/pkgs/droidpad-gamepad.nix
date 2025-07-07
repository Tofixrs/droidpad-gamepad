{
  craneLib,
  pkgs,
  self,
  ...
}:
craneLib.buildPackage {
  pname = "droidpad-gamepad";
  src = craneLib.cleanCargoSource ../..;
  version = "git-${toString (self.shortRev or self.dirtyShortRev or self.lastModified or "unknown")}";
  strictDeps = true;

  nativeBuildInputs = with pkgs; [
    pkg-config
  ];
  buildInputs = with pkgs; [
    libevdev
  ];
}
