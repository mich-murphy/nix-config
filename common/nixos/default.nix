{ lib, config, pkgs, ... }:

{
  imports = [
    ./nextcloud.nix
    ./borgbackup.nix
    ./object-storage.nix
    ./syncthing.nix
    ./plex.nix
    ./freshrss.nix
    ./calibre-web.nix
  ];
}
