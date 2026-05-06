{self, ...}: {
  flake.nixosModules.default = {
    config,
    lib,
    pkgs,
    ...
  }: let
    cfg = config.services.droidpad-gamepad;
  in {
    options.services.droidpad-gamepad = {
      enable = lib.mkEnableOption "Droidpad Gamepad server";
      package = lib.mkOption {
        type = lib.types.package;
        default = self.packages.${pkgs.system}.cli;
        description = "The Droidpad Gamepad package to use.";
      };
    };

    config = lib.mkIf cfg.enable {
      environment.systemPackages = [cfg.package];
      services.udev.packages = [cfg.package];
    };
  };
}
