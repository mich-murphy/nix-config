{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.calibre-web;
in
{
  options.common.calibre-web = {
    enable = mkEnableOption "Enable Calibre-Web";
  };

  config = mkIf cfg.enable {
    services = {
      calibre-web = {
        enable = true;
        listen.ip = "0.0.0.0";
        options = {
          enableBookUploading = true;
          enableBookConversion = true;
          calibreLibrary = "/data/media/books";
        };
      };    
    };
  };
}
