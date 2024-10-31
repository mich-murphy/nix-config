{
  lib,
  pkgs,
  config,
  ...
}: let
  cfg = config.common.netdata;
in {
  options.common.netdata = {
    enable = lib.mkEnableOption "Enable netdata";
  };

  config = lib.mkIf cfg.enable {
    services = {
      netdata = {
        enable = true;
        package = pkgs.netdata.override {
          withCloud = true;
          withCloudUi = true;
        };
        claimTokenFile = config.age.secrets.netdataClaimToken.path;
        python.recommendedPythonPackages = true;
      };
    };

    age.secrets.netdataClaimToken.file = ../../../secrets/netdataClaimToken.age;
  };
}
