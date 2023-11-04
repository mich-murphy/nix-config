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
    ./linkding.nix
    ./uptime-kuma.nix
    ./komga.nix
    ./roon-server.nix
    ./monitoring.nix
    ./openvscode.nix
    ./wallabag.nix
    ./arrs.nix
    ./gitea.nix
  ];
}
