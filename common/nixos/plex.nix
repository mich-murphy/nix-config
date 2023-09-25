{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.plex;
  audnexusPlugin = pkgs.stdenv.mkDerivation {
    name = "Audnexus.bundle";
    src = pkgs.fetchurl {
      url = "https://github.com/djdembeck/Audnexus.bundle/archive/refs/tags/v1.1.0.zip";
      sha256 = "sha256-i5ssEe7SFoQHFXvYiB0nG1mQrcA/wgSeYZiyYKDYtuQ=";
    };
    buildInputs = [ pkgs.unzip ];
    installPhase = "mkdir -p $out; cp -R * $out/";
  };
in
{
  options.common.plex = {
    enable = mkEnableOption "Enable Plex with Audnexus plugin for audiobooks";
    nginx = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable nginx reverse proxy with SSL";
    };
  };

  config = mkIf cfg.enable {
    services = {
      plex = {
        enable = true;
        extraPlugins = [ audnexusPlugin ];
      };
      tautulli.enable = true;
      nginx = mkIf cfg.nginx {
        virtualHosts."plex.pve.elmurphy.com"= {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://127.0.0.1:32400";
            proxyWebsockets = true;
          };
        };
        virtualHosts."tautulli.pve.elmurphy.com"= {
          enableACME = true;
          addSSL = true;
          acmeRoot = null;
          locations."/" = {
            proxyPass = "http://127.0.0.1:8181";
            proxyWebsockets = true;
          };
        };
      };
    };

    environment = {
      systemPackages = with pkgs; [
        linux-firmware 
        intel-gpu-tools
        libva-utils
      ];
    };

    users.users.plex.extraGroups = [ "render" "media" ];
  };
}
