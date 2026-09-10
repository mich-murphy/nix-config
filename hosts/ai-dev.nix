{
  config,
  lib,
  ...
}: {
  imports = [./ai-dev-runtime.nix];

  home = {
    username = "michael";
    homeDirectory = "/home/michael";

    # /tmp is a quota-limited RAM tmpfs; agent scratch belongs on disk.
    sessionVariables.TMPDIR = "/var/tmp/michael";

    # ai-dev is headless: pi-mcp-adapter has no browser or display to open
    # MCP UI sessions in, so keep tool results inline only. Hosts with a
    # browser keep the default viewer behaviour.
    sessionVariables.MCP_UI_VIEWER = "none";
  };

  # Home Manager still uses the deprecated `nix profile install` alias.
  home.activation.installPackages = lib.mkForce (lib.hm.dag.entryAfter ["writeBoundary"] ''
    aiDevNix="$(command -v nix)"
    nixProfileRemove 'home-manager-path'
    run "$aiDevNix" profile add ${config.home.path}
    unset aiDevNix
  '');

  news.display = "silent";

  programs.fish.interactiveShellInit = lib.mkBefore ''
    fish_add_path --prepend "$HOME/.nix-profile/bin"
  '';

  programs.starship.settings = {
    hostname.disabled = true;
    username.disabled = true;
  };

  programs.git.includes = [
    {
      path = "~/.gitconfig-personal";
    }
  ];
}
