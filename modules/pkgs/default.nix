{self, ...}: {
  perSystem = {
    pkgs,
    craneLib,
    craneLibWindows,
    ...
  }: {
    packages = rec {
      default = droidpad-gamepad;
      droidpad-gamepad = import ./droidpad-gamepad.nix {
        inherit pkgs craneLib self;
      };
      windows = import ./droidpad-gamepad.nix {
        pkgs = pkgs.pkgsCross.mingwW64;
        craneLib = craneLibWindows;
        inherit self;
      };
    };
  };
}
