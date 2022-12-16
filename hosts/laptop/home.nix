{ config, pkgs, user, gitUser, gitEmail, ... }:

let
  # fake package - managed by homebrew instead
  fakepkg = name: pkgs.runCommand name {} "mkdir $out";
  monokai-pro = pkgs.vimUtils.buildVimPlugin {
    pname = "monokai-pro";
    version = "1.0.0";
    src = pkgs.fetchFromGitLab {
      owner = "__tpb";
      repo = "monokai-pro.nvim";
      rev = "826d028edbcc7a8aadc0f7a32b32747d97575615";
      sha256 = "UJeg6Kneicf+Mb2BzFHsQYp6U0ol9imBqpIq6QafyFE=";
    };
  };
in
{
  home = {
    username = "${user}";
    homeDirectory = "/Users/${user}";
    stateVersion = "22.05";
    sessionVariables = {
      EDITOR = "nvim";
      VISUAL = "nvim";
      TERMINAL = "alacritty";
    };
    packages = with pkgs; [
      # cli utilities
      fd
      sd
      ripgrep
      jq
      tree
      thefuck
      du-dust
      grex
    ];
  };
  
  programs = {
    home-manager = {
      enable = true;
    };
    direnv = {
      enable = true;
      enableZshIntegration = true;
      nix-direnv.enable = true;
    };
    git = {
      enable = true;
      userName = "${gitUser}";
      userEmail = "${gitEmail}";
      extraConfig = {
        init.defaultBranch = "main";
        pull.rebase = true;
      };
      diff-so-fancy.enable = true;
    };
    zsh = {
      enable = true;
      enableAutosuggestions = true;
      enableSyntaxHighlighting = true;
      enableCompletion = true;
      defaultKeymap = "viins";
      history.size = 10000;
      initExtra = ''
        eval $(thefuck --alias)
      '';
      shellAliases = {
        ls = "lsd -lah";
        cat = "bat";
        gs = "git status";
        ga = "git add";
        gc = "git commit";
        gh = "git pull";
        gp = "git push";
        gpl = "git pull";
        gb = "git branch";
        gch = "git checkout";
      };
    };
    starship = {
      enable = true;
      enableZshIntegration = true;
      settings = {
        scan_timeout = 10;
      };
    };
    zoxide = {
      enable = true;
      enableZshIntegration = true;
    };
    bat = {
      enable = true;
      config = {
        theme = "Monokai Extended Origin";
      };
      extraPackages = with pkgs.bat-extras; [
        batgrep
        batdiff
      ];
    };
    tealdeer = {
      enable = true;
      settings = {
        display = {
          compact = true;
        };
      };
    };
    fzf = {
      enable = true;
      enableZshIntegration = true;
    };
    tmux = {
      enable = true;
      baseIndex = 1;
      customPaneNavigationAndResize = true;
      disableConfirmationPrompt = true;
      escapeTime = 10;
      historyLimit = 10000;
      keyMode = "vi";
      newSession = true;
      prefix = "C-Space";
      terminal = "screen-256color";
      tmuxp.enable = true;
      plugins = with pkgs; [
        tmuxPlugins.tmux-fzf
        {
          plugin = tmuxPlugins.power-theme;
          extraConfig = "set -g @tmux_power_theme 'moon'";
        }
      ];
      extraConfig = ''
        # Vim settings
        set-option -g focus-events on
        # Changing key bindings
        unbind r
        bind r source-file $XDG_CONFIG_HOME/tmux/tmux.conf \; display "Reloaded tmux conf"
        set -g mouse on
        unbind v
        unbind h
        unbind %
        unbind '"'
        bind v split-window -h -c "#{pane_current_path}"
        bind h split-window -v -c "#{pane_current_path}"
        bind -n C-h select-pane -L
        bind -n C-j select-pane -D
        bind -n C-k select-pane -U
        bind -n C-l select-pane -R
        unbind n
        unbind w
        bind n command-prompt "rename-window '%%'"
        bind w new-window -c "#{pane_current_path}"
        bind -n M-j previous-window
        bind -n M-k next-window
        # Vim keybindings
        unbind -T copy-mode-vi Space;
        unbind -T copy-mode-vi Enter;
        bind -T copy-mode-vi v send-keys -X begin-selection
        bind -T copy-mode-vi y send-keys -X copy-pipe-and-cancel "xsel --clipboard" 
        set -g -a terminal-overrides ',*:Ss=\E[%p1%d q:Se=\E[2 q'
        is_vim="ps -o state= -o comm= -t '#{pane_tty}' \
            | grep -iqE '^[^TXZ ]+ +(\\S+\\/)?g?(view|n?vim?x?)(diff)?$'"
        bind -n C-h if-shell "$is_vim" "send-keys C-h"  "select-pane -L"
        bind -n C-j if-shell "$is_vim" "send-keys C-j"  "select-pane -D"
        bind -n C-k if-shell "$is_vim" "send-keys C-k"  "select-pane -U"
        bind -n C-l if-shell "$is_vim" "send-keys C-l"  "select-pane -R"
        bind -n C-\\ if-shell "$is_vim" "send-keys C-\\" "select-pane -l"
      '';
    };
    btop = {
      enable = true;
      settings = {
        color_theme = "TTY";
        theme_background = false;
        vim_keys = true;
      };
    };
    lsd = {
      enable = true;
      settings = {
        classic = false;
	blocks = [
          "permission"
	  "user"
	  "size"
	  "date"
	  "name"
	];
	date = "+%d %b %R";
	icons = {
          when = "auto";
	  theme = "fancy";
	  separator = " ";
	};
	layout = "grid";
	sorting = {
          column = "name";
	  reverse = false;
	  dir-grouping = "first";
	};
        symlink-arrow = "->";
      };
    };
    lf = {
      enable = true;
      keybindings = {
        DD = "delete";
        p = "paste";
        x = "cut";
        y = "copy";
        l = "open";
        c = "clear";
        gn = "cd ~/nix-config";
        gg = "cd ~/git";
        gd = "cd ~/Downloads";
        gD = "cd ~/Documents";
        gp = "cd ~/Pictures";
        gc = "cd ~/.config";
      };
      settings = {
        hidden = true;
        dirfirst = true;
        relativenumber = true;
        ignorecase = true;
        globsearch = true;
        scrolloff = 8;
      };
    extraConfig = "set previewer /etc/profiles/per-user/mm/bin/pistol";
    };
    alacritty = {
      enable = true;
      package = fakepkg "alacritty";
      settings = {
        live_config_reload = true;
        dynamic_title = true;
        window = {
	  decorations = "buttonless";
	  padding = {
            x = 15;
            y = 15;
          };
	};
        font = {
          size = 13.0; 
          normal = {
            family = "JetBrainsMono Nerd Font";
            style = "Regular";
          };
          bold = {
            family = "JetBrainsMono Nerd Font";
            style = "Bold";
          };
          italic = {
            family = "JetBrainsMono Nerd Font";
            style = "Italic";
          };
        };
        draw_bold_text_with_bright_colors = true;
        colors = {
          primary = {
            background = "#222222";
            foreground = "#f7f1ff";
          };
          selection = {
            text = "#bab6c0";
            background = "#403e41";
          };
          normal = {
            black = "#363537";
            red = "#fc618d";
            green = "#7db88f";
            yellow = "#fce566";
            blue = "0x61afef";
            magenta = "#948ae3";
            cyan = "#5ad4e6";
            white = "#f7f1ff";
          }; 
          bright = {
            black = "#403e41";
            red = "#fc618d";
            green = "#7db88f";
            yellow = "#fce566";
            blue = "0x61afef";
            magenta = "#948ae3";
            cyan = "#5ad4e6";
            white = "#f7f1ff";
          }; 
	};
      };
    };   
    firefox = {
      enable = true;
      # fake package - managed by homebrew instead
      package = fakepkg "firefox";
      extensions = with pkgs.nur.repos.rycee.firefox-addons; [
        ublock-origin
        onepassword-password-manager
        vimium
        decentraleyes
        privacy-badger
        sponsorblock
        new-tab-override
      ];
      profiles."${user}" = {
        isDefault = true;
        settings = {
          # Configured via Firefox Profilemaker
          "app.normandy.api_url" = "";
          "app.normandy.enabled" = false;
          "app.shield.optoutstudies.enabled" = false;
          "app.update.auto" = false;
          "beacon.enabled" = false;
          "breakpad.reportURL" = "";
          "browser.aboutConfig.showWarning" = false;
          "browser.crashReports.unsubmittedCheck.autoSubmit" = false;
          "browser.crashReports.unsubmittedCheck.autoSubmit2" = false;
          "browser.crashReports.unsubmittedCheck.enabled" = false;
          "browser.disableResetPrompt" = true;
          "browser.newtabpage.activity-stream.section.highlights.includePocket" = false;
          "browser.newtabpage.enhanced" = false;
          "browser.newtabpage.introShown" = true;
          "browser.safebrowsing.appRepURL" = "";
          "browser.safebrowsing.blockedURIs.enabled" = false;
          "browser.safebrowsing.downloads.enabled" = false;
          "browser.safebrowsing.downloads.remote.enabled" = false;
          "browser.safebrowsing.downloads.remote.url" = "";
          "browser.safebrowsing.enabled" = false;
          "browser.safebrowsing.malware.enabled" = false;
          "browser.safebrowsing.phishing.enabled" = false;
          "browser.selfsupport.url" = "";
          "browser.sessionstore.privacy_level" = 2;
          "browser.shell.checkDefaultBrowser" = false;
          "browser.startup.homepage_override.mstone" = "ignore";
          "browser.tabs.crashReporting.sendReport" = false;
          "browser.urlbar.groupLabels.enabled" = false;
          "browser.urlbar.quicksuggest.enabled" = false;
          "browser.urlbar.trimURLs" = false;
          "datareporting.healthreport.service.enabled" = false;
          "datareporting.healthreport.uploadEnabled" = false;
          "datareporting.policy.dataSubmissionEnabled" = false;
          "device.sensors.ambientLight.enabled" = false;
          "device.sensors.enabled" = false;
          "device.sensors.motion.enabled" = false;
          "device.sensors.orientation.enabled" = false;
          "device.sensors.proximity.enabled" = false;
          "dom.battery.enabled" = false;
          "dom.security.https_only_mode" = true;
          "dom.security.https_only_mode_ever_enabled" = true;
          "experiments.activeExperiment" = false;
          "experiments.enabled" = false;
          "experiments.manifest.uri" = "";
          "experiments.supported" = false;
          "extensions.getAddons.cache.enabled" = false;
          "extensions.getAddons.showPane" = false;
          "extensions.pocket.enabled" = false;
          "extensions.shield-recipe-client.api_url" = "";
          "extensions.shield-recipe-client.enabled" = false;
          "extensions.webservice.discoverURL" = "";
          "media.autoplay.default" = 1;
          "media.autoplay.enabled" = false;
          "media.navigator.enabled" = false;
          "network.allow-experiments" = false;
          "network.cookie.cookieBehavior" = 1;
          # testing for paypal issues
          "network.http.referer.spoofSource" = false;
          "privacy.query_stripping" = true;
          "privacy.trackingprotection.cryptomining.enabled" = true;
          "privacy.trackingprotection.enabled" = true;
          "privacy.trackingprotection.fingerprinting.enabled" = true;
          "privacy.trackingprotection.pbmode.enabled" = true;
          "privacy.usercontext.about_newtab_segregation.enabled" = true;
          "services.sync.prefs.sync.browser.newtabpage.activity-stream.showSponsoredTopSite" = false;
          "signon.autofillForms" = false;
          "toolkit.telemetry.archive.enabled" = false;
          "toolkit.telemetry.bhrPing.enabled" = false;
          "toolkit.telemetry.cachedClientID" = "";
          "toolkit.telemetry.enabled" = false;
          "toolkit.telemetry.firstShutdownPing.enabled" = false;
          "toolkit.telemetry.hybridContent.enabled" = false;
          "toolkit.telemetry.newProfilePing.enabled" = false;
          "toolkit.telemetry.prompted" = 2;
          "toolkit.telemetry.rejected" = true;
          "toolkit.telemetry.reportingpolicy.firstRun" = false;
          "toolkit.telemetry.server" = "";
          "toolkit.telemetry.shutdownPingSender.enabled" = false;
          "toolkit.telemetry.unified" = false;
          "toolkit.telemetry.unifiedIsOptIn" = false;
          "toolkit.telemetry.updatePing.enabled" = false;
          "webgl.renderer-string-override" = " ";
          "webgl.vendor-string-override" = " ";
        };
        userChrome = builtins.readFile ../../config/firefox/userChrome.css;
      };
    };
    neovim = {
      enable = true;
      package = pkgs.neovim-nightly;
      viAlias = true;
      vimAlias = true;
      withPython3 = true;
      plugins = with pkgs.vimPlugins; [ 
        vim-surround
        vim-bbye
        nvim-autopairs
        vim-sleuth
        nvim-web-devicons
        indent-blankline-nvim
        lualine-nvim
        impatient-nvim
        project-nvim
        nvim-tree-lua
        nvim-colorizer-lua
        vim-nix
        # lsp
        nvim-lspconfig
        null-ls-nvim
        vim-illuminate
        # cmp
        nvim-cmp
        cmp_luasnip
        cmp-path
        cmp-buffer
        cmp-nvim-lsp
        cmp-nvim-lua
        lspkind-nvim
        # snippets
        luasnip
        friendly-snippets
        # colorscheme
        monokai-pro
        # telescope
        telescope-nvim
        plenary-nvim
        telescope-file-browser-nvim
        telescope-fzf-native-nvim
        # treesitter - with grammars loaded
        (nvim-treesitter.withPlugins (plugins: pkgs.tree-sitter.allGrammars))
        nvim-treesitter-context
        nvim-treesitter-textobjects
        nvim-ts-context-commentstring
        # comments
        comment-nvim
        # git
        gitsigns-nvim
        vim-fugitive
        # tmux
        vim-tmux-navigator
        tmux-complete-vim
        vim-tmux
        # DAP
        nvim-dap
        nvim-dap-ui
      ];
      extraConfig = "luafile ~/.config/nvim/settings.lua";
      extraPackages = with pkgs; [
        (python310.withPackages (ps: with ps; [
        # python linting/formatting
        black
        flake8
        # debugger
        debugpy
        ]))
        # formatters
        nixfmt
        stylua
        # language servers
        sqls
        rnix-lsp
        nodePackages.pyright
        sumneko-lua-language-server
        nodePackages.yaml-language-server
        nodePackages.bash-language-server
        nodePackages_latest.vscode-langservers-extracted
      ];
    };
  };

  xdg.configFile = {
    nvim = {
      source = ../../config/nvim;
      target =  "nvim";
      recursive = true;
    };
  };
}
