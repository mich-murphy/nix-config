{...}: {
  imports = [
    ./nextcloud.nix
    ./borgbackup.nix
    ./freshrss.nix
    ./tailscale.nix
    ./acme.nix
    ./gitea.nix
    ./media/plex.nix
    ./media/audiobookshelf.nix
    ./media/jellyfin.nix
    ./media/komga.nix
    ./media/ytdlp.nix
    ./games/murmur.nix
    ./games/minecraft.nix
    ./downloads/sabnzdb.nix
    ./downloads/lidarr.nix
    ./downloads/radarr.nix
    ./downloads/sonarr.nix
    ./downloads/readarr.nix
    ./downloads/prowlarr.nix
  ];
}
