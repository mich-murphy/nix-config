# dev-01: Ubuntu with standalone Home Manager. The machines
# repo's bootstrap owns everything outside the home directory, the agent CLIs
# and ~/.gitconfig-work.
{ pkgs, ... }:
{
  home = {
    username = "dev";
    homeDirectory = "/home/dev";
    # Set once at first activation; never bump it to track releases.
    stateVersion = "26.05";
    # Codex's sandbox on Linux.
    packages = [ pkgs.bubblewrap ];
    # Node's os.tmpdir() and audit caches can reach hundreds of MiB, so keep
    # them on the data disk; bootstrap creates this directory after mounting it.
    sessionVariables.TMPDIR = "/home/dev/businesscraft/.scratch";
  };

  # Nixpkgs' systemctl cannot reach Ubuntu's older user manager, so activation
  # would report it as not running and skip restarting changed services.
  systemd.user.systemctlPath = "/usr/bin/systemctl";

  # Work identity, written by bootstrap.
  programs.git.includes = [ { path = "~/.gitconfig-work"; } ];

  # Standalone hosts switch with the home-manager command.
  programs.home-manager.enable = true;
}
