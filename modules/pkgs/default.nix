{self, ...}: {
  perSystem = {
    pkgs,
    craneLib,
    ...
  }: {
    packages = rec {
      default = droidpad-gamepad;
      droidpad-gamepad = import ./droidpad-gamepad.nix {
        inherit pkgs craneLib self;
      };
    };
  };
}
