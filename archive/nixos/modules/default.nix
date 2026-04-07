{...}: {
  imports = [
    ./downloads
    ./media
    ./monitoring
    ./acme.nix
    ./borgbackup.nix
    ./freshrss.nix
    ./forgejo.nix
    ./matrix.nix
    ./tailscale.nix
    ./beszel.nix
    ./searxng.nix
    ./smokeping.nix
    ./paperless.nix
  ];
}
