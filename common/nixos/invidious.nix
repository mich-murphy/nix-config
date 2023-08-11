{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.invidious;
in
{
  options.common.invidious = {
    enable = mkEnableOption "Enable Invidious service";
    port = mkOption {
      type = types.port;
      default = 3000;
      description = "Port for Invidious to be advertised on";
    };
  };

  config = mkIf cfg.enable {
    services = {
      invidious = {
        enable = true;
        domain = "invidious.pve.elmurphy.com";
        port = cfg.port;
        nginx.enable = true;
      };
    };
  };
 }
