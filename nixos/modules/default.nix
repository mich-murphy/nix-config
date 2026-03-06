{...}: {
  imports = [
    ./downloads
    ./games
    ./media
    ./monitoring
    ./acme.nix
    ./borgbackup.nix
    ./freshrss.nix
    ./forgejo.nix
    ./matrix.nix
    ./tailscale.nix
    ./beszel.nix
    ./it-tools.nix
    ./actual.nix
    ./searxng.nix
    ./gitlab.nix
    ./n8n.nix
    ./stirling-pdf.nix
    ./smokeping.nix
    ./rss-bridge.nix
    ./paperless.nix
  ];
}
