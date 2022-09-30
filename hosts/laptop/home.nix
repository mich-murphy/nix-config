
{ config, pkgs, user, gitUser, gitEmail, ... }:

let
  alpha-nvim = pkgs.vimUtils.buildVimPlugin {
    name = "alpha-nvim";
    src = pkgs.fetchFromGitHub {
      owner = "goolord";
      repo = "alpha-nvim";
      rev = "0bb6fc0646bcd1cdb4639737a1cee8d6e08bcc31";
      sha256 = "tKXSFZusajLLsbQj6VKZG1TJB+i5i1H5e5Q5tbe+ojM=";
    };
  };
  monokai-pro = pkgs.vimUtils.buildVimPlugin {
    name = "monokai-pro";
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
      TERMINAL = "kitty";
    };
    packages = with pkgs; [
      # shell prompt
      starship
      # cli utilities
      fd
      fzf
      ripgrep
      bat
      lsd
      jq
      wget
      tree
      btop
      spotify-tui
      ranger
      lazygit
      thefuck
    ];
  };

  programs = {
    home-manager = {
      enable = true;
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
        eval "$(starship init zsh)"
      '';
      shellAliases = {
        ls = "lsd -lah";
        cat = "bat";
        spt = "spotifyd &; spt";
        gs = "git status";
        ga = "git add";
        gc = "git commit";
        gh = "git pull";
        gp = "git push";
        nix = "cd ~/nix-config";
      };
    };
    fzf = {
      enable = true;
      enableZshIntegration = true;
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
    alacritty = {
      enable = true;
      # fake package - managed by homebrew instead
      package = pkgs.runCommand "alacritty-0.0.0" {} "mkdir $out";
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
      package = pkgs.runCommand "firefox-0.0.0" {} "mkdir $out";
      extensions = with pkgs.nur.repos.rycee.firefox-addons; [
        ublock-origin
        onepassword-password-manager
        vimium
        decentraleyes
        privacy-badger
        darkreader
        sponsorblock
      ];
      profiles."${user}" = {
        isDefault = true;
        settings = {
          "browser.send_pings" = false;
          "browser.urlbar.speculativeConnect.enabled" = false;
          "dom.event.clipboardevents.enabled" = true;
          "media.navigator.enabled" = false;
          "network.cookie.cookieBehavior" = 1;
          "network.http.referer.XOriginPolicy" = 2;
          "network.http.referer.XOriginTrimmingPolicy" = 2;
          "beacon.enabled" = false;
          "browser.safebrowsing.downloads.remote.enabled" = false;
          "network.IDN_show_punycode" = true;
          "extensions.activeThemeID" = "firefox-compact-dark@mozilla.org";
          "app.shield.optoutstudies.enabled" = false;
          "dom.security.https_only_mode_ever_enabled" = true;
          "toolkit.legacyUserProfileCustomizations.stylesheets" = true;
          "browser.toolbars.bookmarks.visibility" = "never";
          "geo.enabled" = false;
          # Disable telemetry
          "browser.newtabpage.activity-stream.feeds.telemetry" = false;
          "browser.ping-centre.telemetry" = false;
          "browser.tabs.crashReporting.sendReport" = false;
          "devtools.onboarding.telemetry.logged" = false;
          "toolkit.telemetry.enabled" = false;
          "toolkit.telemetry.unified" = false;
          "toolkit.telemetry.server" = "";
          # Disable Pocket
          "browser.newtabpage.activity-stream.feeds.discoverystreamfeed" = false;
          "browser.newtabpage.activity-stream.feeds.section.topstories" = false;
          "browser.newtabpage.activity-stream.section.highlights.includePocket" = false;
          "browser.newtabpage.activity-stream.showSponsored" = false;
          "extensions.pocket.enabled" = false;
          # Disable prefetching
          "network.dns.disablePrefetch" = true;
          "network.prefetch-next" = false;
          # Disable JS in PDFs
          "pdfjs.enableScripting" = false;
          # Harden SSL 
          "security.ssl.require_safe_negotiation" = true;
          # Extra
          "identity.fxaccounts.enabled" = false;
          "browser.search.suggest.enabled" = false;
          "browser.urlbar.shortcuts.bookmarks" = false;
          "browser.urlbar.shortcuts.history" = false;
          "browser.urlbar.shortcuts.tabs" = false;
          "browser.urlbar.suggest.bookmark" = true;
          "browser.urlbar.suggest.engines" = false;
          "browser.urlbar.suggest.history" = true;
          "browser.urlbar.suggest.openpage" = true;
          "browser.urlbar.suggest.topsites" = false;
          "browser.uidensity" = 1;
          "media.autoplay.enabled" = false;
          "toolkit.zoomManager.zoomValues" = ".8,.90,.95,1,1.1,1.2";
          "privacy.firstparty.isolate" = true;
          "network.http.sendRefererHeader" = 0;
        };
        userChrome = builtins.readFile ../../config/firefox/userChrome.css;
      };
    };
    neovim = {
      enable = true;
      package = pkgs.neovim;
      viAlias = true;
      vimAlias = true;
      withPython3 = true;
      plugins = with pkgs.vimPlugins; [ 
        vim-surround
        vim-bbye
        nvim-autopairs
        nvim-web-devicons
        indent-blankline-nvim
        bufferline-nvim
        lualine-nvim
        toggleterm-nvim
        impatient-nvim
        project-nvim
        alpha-nvim
        nvim-tree-lua
        dressing-nvim
        nvim-colorizer-lua
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
        # snippets
        luasnip
        friendly-snippets
        # colorscheme
        {
          plugin = monokai-pro;
          config = ''
            lua <<EOF
              vim.g.monokaipro_filter = "spectrum"
              vim.g.monokaipro_flat_sidebar = true
              vim.g.monokaipro_flat_float = true
              vim.g.monokaipro_flat_term = true
              vim.cmd[[colorscheme monokaipro]]
            EOF
          '';
        }
        # telescope
        telescope-nvim
        plenary-nvim
        telescope-file-browser-nvim
        telescope-fzf-native-nvim
        # treesitter - could not load parsers with with external config
        {
          plugin = nvim-treesitter.withPlugins (plugins: pkgs.tree-sitter.allGrammars);
          config = ''
            lua <<EOF
            require('nvim-treesitter.configs').setup {
              highlight = {
                enable = true,
                additional_vim_regex_highlighting = false,
              },
              indent = {
                enable = true,
                disable = { "python", "css" }
              },
              autopairs = {
                enable = true,
              },
            }
            EOF
          '';
        }
        nvim-treesitter-context
        vim-nix
        # comments
        comment-nvim
        nvim-ts-context-commentstring
        # git
        gitsigns-nvim
        vim-fugitive
      ];
      extraConfig = "luafile ~/.config/nvim/settings.lua";
      extraPackages = with pkgs; [
        rnix-lsp
        nixfmt
        sumneko-lua-language-server
        stylua
        nodePackages.pyright
      ];
      extraPython3Packages = (ps: with ps; [
        jedi
        pynvim
        pkgs.python310Packages.python-lsp-server
        pkgs.python310Packages.python-lsp-black
      ]);
    };
  };

  xdg.configFile = {
    nvim = {
      source = ../../config/nvim;
      target =  "nvim";
      recursive = true;
    };
    "ranger/rc.conf".source = ../../config/ranger/rc.conf;
    "karabiner/karabiner.json".source = ../../config/karabiner/karabiner.json;
  };
}
