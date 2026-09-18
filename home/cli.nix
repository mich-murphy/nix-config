{pkgs, ...}: let
  # Upstream nixpkgs bug: the containerapp wheel pins kubernetes==24.2.0
  # while the azure-cli python env ships kubernetes 35.x, failing the strict
  # runtime deps check. The kubernetes client is only used by a few containerapp
  # subcommands (compose, kubernetes-yaml), so skip the check until nixpkgs
  # catches up. Note: withExtensions also bundles pip, so `az extension add`
  # keeps working for extensions not packaged in nixpkgs.
  containerapp = pkgs.azure-cli.extensions.containerapp.overrideAttrs (oldAttrs: {
    dontCheckRuntimeDeps = true;
  });
  azureCliWithExtensions = pkgs.azure-cli.withExtensions [containerapp];
  btopWithManpage = pkgs.btop.overrideAttrs (previousAttrs: {
    nativeBuildInputs =
      (previousAttrs.nativeBuildInputs or [])
      ++ [pkgs.lowdown-unsandboxed];
  });
in {
  home.packages = [
    azureCliWithExtensions
    pkgs.cargo
    pkgs.curl
    pkgs.doctl
    pkgs.dust
    pkgs.fd
    pkgs.go
    pkgs.just
    pkgs.jq
    pkgs.nodejs
    pkgs.opencode
    pkgs.ouch
    pkgs.python3
    pkgs.procs
    pkgs.prek
    pkgs.ripgrep
    pkgs.rsync
    pkgs.sd
    pkgs.tree
    pkgs.uv
    pkgs.wget
  ];

  programs = {
    direnv = {
      enable = true;
      enableFishIntegration = true;
      nix-direnv.enable = true;
    };
    bat = {
      enable = true;
      config = {
        theme = "tokyonight_night";
      };
      themes.tokyonight_night = {
        src = pkgs.vimPlugins.tokyonight-nvim;
        file = "extras/sublime/tokyonight_night.tmTheme";
      };
    };
    starship = {
      enable = true;
      enableFishIntegration = true;
      settings = {
        scan_timeout = 10;
        git_status = {
          deleted = "";
        };
      };
    };
    eza = {
      enable = true;
      icons = "auto";
      extraOptions = [
        "--group-directories-first"
      ];
    };
    zoxide = {
      enable = true;
      enableFishIntegration = true;
    };
    tealdeer = {
      enable = true;
      enableAutoUpdates = false;
      settings = {
        display = {
          compact = true;
        };
      };
    };
    btop = {
      enable = true;
      package = btopWithManpage;
      settings = {
        color_theme = "TTY";
        theme_background = false;
        vim_keys = true;
      };
    };
    fzf = {
      enable = true;
      tmux.enableShellIntegration = true;
      fileWidget.command = ''rg --files --hidden --glob "!.git"'';
      colors = {
        "bg+" = "#1a1b26";
        fg = "#a9b1d6";
        "fg+" = "#c0caf5";
        border = "#1a1b26";
        spinner = "#3b4261";
        hl = "#7dcfff";
        header = "#e0af68";
        info = "#7aa2f7";
        pointer = "#7aa2f7";
        marker = "#f7768e";
        prompt = "#a9b1d6";
        "hl+" = "#7aa2f7";
      };
      defaultOptions = [
        "--height 60%"
        "--border none"
        "--layout reverse"
      ];
      fileWidget.options = [
        "--height 60%"
        "--border none"
        "--no-scrollbar"
        "--inline-info"
        "--layout reverse"
        "--preview 'bat --color=always {}'"
        "--preview-window '~2',border-none"
      ];
      changeDirWidget.options = [
        "--preview 'tree -C {} | head -n 10'"
      ];
    };
  };
}
