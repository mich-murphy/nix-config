# dev-01 (SSH alias bc-01): Ubuntu with standalone Home Manager. The machines
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
  };

  # Work identity, written by bootstrap.
  programs.git.includes = [ { path = "~/.gitconfig-work"; } ];

  # Standalone hosts switch with the home-manager command.
  programs.home-manager.enable = true;
}
