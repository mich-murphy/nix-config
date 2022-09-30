
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
            background = "0x282c34";
            foreground = "0xabb2bf";
          };
          cursor = {
            text = "0x2c323c";
            cursor = "0x5c6370";
          };
          selection = {
            text = "CellForeground";
            background = "0x3e4452";
          };
          normal = {
            black = "0x2c323c";
            red = "0xe06c75";
            green = "0x98c379";
            yellow = "0xe5c07b";
            blue = "0x61afef";
            magenta = "0xc678dd";
            cyan = "0x56b6c2";
            white = "0x5c6370";
          }; 
          bright = {
            black = "0x3e4452";
            red = "0xe06c75";
            green = "0x98c379";
            yellow = "0xe5c07b";
            blue = "0x61afef";
            magenta = "0xc678dd";
            cyan = "0x56b6c2";
            white = "0xabb2bf";
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
        vim-sleuth
        vim-bbye
        nvim-autopairs
        comment-nvim
        nvim-ts-context-commentstring
        nvim-web-devicons
        indent-blankline-nvim
        bufferline-nvim
        lualine-nvim
        toggleterm-nvim
        impatient-nvim
        project-nvim
        alpha-nvim
        nvim-tree-lua
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
        tokyonight-nvim
        { 
          plugin = onedark-nvim;
          config = ''
            lua <<EOF
            require('onedark').setup {
                style = 'darker'
            }
            require('onedark').load()
            EOF
          '';
        }
        # telescope
        telescope-nvim
        plenary-nvim
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
