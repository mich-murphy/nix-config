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
    ./audiobookshelf.nix
    ./tailscale.nix
    ./jellyfin.nix
    ./nginx.nix
  ];
}
