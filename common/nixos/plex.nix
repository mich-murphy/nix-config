{ lib, config, pkgs, ... }:

with lib;

let
  cfg = config.common.plex;
  audnexusPlugin = pkgs.stdenv.mkDerivation {
    name = "Audnexus.bundle";
    src = pkgs.fetchurl {
      url = https://github.com/djdembeck/Audnexus.bundle/archive/refs/tags/v1.1.0.zip;
      sha256 = "sha256-i5ssEe7SFoQHFXvYiB0nG1mQrcA/wgSeYZiyYKDYtuQ=";
    };
    buildInputs = [ pkgs.unzip ];
    installPhase = "mkdir -p $out; cp -R * $out/";
  };
in
{
  options.common.plex = {
    enable = mkEnableOption "Enable Plex with Audnexus plugin for audiobooks";
  };

  config = mkIf cfg.enable {
      services.plex = {
      enable = true;
      extraPlugins = [ audnexusPlugin ];
    };
  };
}
