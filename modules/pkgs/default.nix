_: {
  perSystem = {
    pkgs,
    craneLib,
    ...
  }: {
    packages = {
      droidpad-gamepad = import ./droidpad-gamepad.nix {
        inherit pkgs craneLib;
      };
    };
  };
}
