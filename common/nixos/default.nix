{...}: {
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
    ./acme.nix
    ./uptime-kuma.nix
    ./komga.nix
    ./monitoring.nix
    ./wallabag.nix
    ./arrs.nix
    ./gitea.nix
    ./nginx.nix
  ];
}
