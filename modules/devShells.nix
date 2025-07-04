_: {
  perSystem = {
    self',
    pkgs,
    ...
  }: {
    devShells.default = pkgs.mkShell {
      name = "shell";
      inputsFrom = [
        self'.packages.droidpad-gamepad
      ];
      packages = with pkgs; [
        rust-analyzer
      ];
    };
  };
}
