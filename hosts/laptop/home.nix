{ config, pkgs, user, gitUser, gitEmail, ... }:

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
      # shell prompt
      starship
      # cli utilities
      ranger
      fd
      ripgrep
      bat
      lsd
      jq
      wget
      tree
      btop
      spotify-tui
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
        init = { defaultBranch = "main"; };
        pull = { rebase = true; };
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
      dirHashes = {
        nx = "$HOME/nix-config";
        dl = "$HOME/Downloads";
      };
      shellAliases = {
        ls = "lsd -lah";
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
	  decorations = "none";
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
            background = "0x2b2d3a";
            foreground = "0xc5cdd9";
          };
          normal = {
            black = "0x363a4e";
            red = "0xec7279";
            green = "0xa0c980";
            yellow = "0xdeb974";
            blue = "0x6cb6eb";
            magenta = "0xd38aea";
            cyan = "0x5dbbc1";
            white = "0xc5cdd9";
          }; 
          bright = {
            black = "0x363a4e";
            red = "0xec7279";
            green = "0xa0c980";
            yellow = "0xdeb974";
            blue = "0x6cb6eb";
            magenta = "0xd38aea";
            cyan = "0x5dbbc1";
            white = "0xc5cdd9";
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
        userChrome = ''
          * { 
              box-shadow: none !important;
              border: 0px solid !important;
          }
          #tabbrowser-tabs {
              --user-tab-rounding: 8px;
          }
          .tab-background {
              border-radius: var(--user-tab-rounding) var(--user-tab-rounding) 0px 0px !important; /* Connected */
              margin-block: 1px 0 !important; /* Connected */
          }
          #scrollbutton-up, #scrollbutton-down { /* 6/10/2021 */
              border-top-width: 1px !important;
              border-bottom-width: 0 !important;
          }
          .tab-background:is([selected], [multiselected]):-moz-lwtheme {
              --lwt-tabs-border-color: rgba(0, 0, 0, 0.5) !important;
              border-bottom-color: transparent !important;
          }
          [brighttext='true'] .tab-background:is([selected], [multiselected]):-moz-lwtheme {
              --lwt-tabs-border-color: rgba(255, 255, 255, 0.5) !important;
              border-bottom-color: transparent !important;
          }
          /* Container color bar visibility */
          .tabbrowser-tab[usercontextid] > .tab-stack > .tab-background > .tab-context-line {
              margin: 0px max(calc(var(--user-tab-rounding) - 3px), 0px) !important;
          }
          #TabsToolbar, #tabbrowser-tabs {
              --tab-min-height: 29px !important;
          }
          #main-window[sizemode='true'] #toolbar-menubar[autohide='true'] + #TabsToolbar, 
          #main-window[sizemode='true'] #toolbar-menubar[autohide='true'] + #TabsToolbar #tabbrowser-tabs {
              --tab-min-height: 30px !important;
          }
          #scrollbutton-up,
          #scrollbutton-down {
              border-top-width: 0 !important;
              border-bottom-width: 0 !important;
          }
          #TabsToolbar, #TabsToolbar > hbox, #TabsToolbar-customization-target, #tabbrowser-arrowscrollbox  {
              max-height: calc(var(--tab-min-height) + 1px) !important;
          }
          #TabsToolbar-customization-target toolbarbutton > .toolbarbutton-icon, 
          #TabsToolbar-customization-target .toolbarbutton-text, 
          #TabsToolbar-customization-target .toolbarbutton-badge-stack,
          #scrollbutton-up,#scrollbutton-down {
              padding-top: 7px !important;
              padding-bottom: 6px !important;
          }
        '';
      };
    };
    zathura = {
      enable = true;
      package = pkgs.zathura;
      extraConfig = ''
	set recolor
	set guioptions ""
	set recolor-lightcolor \#1f2227
	set default-bg \#1f2227
	set adjust-open "best-fit"
	set scroll-page-aware "true"
	set sandbox none
	set statusbar-h-padding 0
	set statusbar-v-padding 0
	map K zoom in
	map R rotate
	map r reload
	map J zoom out
	set selection-clipboard clipboard
        set notification-error-bg       "#2b2d3a" # bg0
        set notification-error-fg       "#ec7279" # red
        set notification-warning-bg     "#2b2d3a" # bg0
        set notification-warning-fg     "#deb974" # yellow
        set notification-bg             "#2b2d3a" # bg0
        set notification-fg             "#a0c980" # green
        set completion-bg               "#363a4e" # bg2
        set completion-fg               "#c5cdd9" # fg0
        set completion-group-bg         "#333648" # bg1
        set completion-group-fg         "#7e8294" # gray
        set completion-highlight-bg     "#6cb6eb" # blue
        set completion-highlight-fg     "#363a4e" # bg2
        # Define the color in index mode
        set index-bg                    "#363a4e" # bg2
        set index-fg                    "#c5cdd9" # fg0
        set index-active-bg             "#6cb6eb" # blue
        set index-active-fg             "#363a4e" # bg2
        set inputbar-bg                 "#363a4e" # bg2
        set inputbar-fg                 "#c5cdd9" # fg0
        set statusbar-bg                "#363a4e" # bg2
        set statusbar-fg                "#c5cdd9" # fg0
        set highlight-color             "#deb974" # yellow
        set highlight-active-color      "#4e432f" # orange
        set default-bg                  "#2b2d3a" # bg0
        set default-fg                  "#c5cdd9" # fg1
        set render-loading              true
        set render-loading-bg           "#2b2d3a" # bg0
        set render-loading-fg           "#c5cdd9" # fg0
        # Recolor book content's color
        set recolor-lightcolor          "#2b2d3a" # bg0
        set recolor-darkcolor           "#c5cdd9" # fg0
        set recolor                     "true"
        set recolor-keephue             true      # keep original color
      '';
    };
    neovim = {
      enable = true;
      package = pkgs.neovim-unwrapped;
      viAlias = true;
      vimAlias = true;
      plugins = with pkgs.vimPlugins; [ 
        vim-fugitive
        vim-nix
        vim-rhubarb
        {
          plugin = gitsigns-nvim;
          config = ''
            lua << EOF
            require('gitsigns').setup {
              signs = {
                add = { text = '+' },
                change = { text = '~' },
                delete = { text = '_' },
                topdelete = { text = '‾' },
                changedelete = { text = '~' },
              },
            }
            EOF
          '';
        }
        plenary-nvim
        {
          plugin = nvim-comment;
          config = "lua require('Comment').setup()";
        }
        {
          plugin = nvim-treesitter;
          config = ''
            lua << EOF
            require('nvim-treesitter.configs').setup {
              highlight = {
                enable = true,
                additional_vim_regex_highlighting = false,
              },
            }
            EOF
          '';
        }
        nvim-treesitter-textobjects
        {
          plugin = nvim-lspconfig;
          config = ''
            lua << EOF
            require('lspconfig').rust_analyzer.setup{}
            require('lspconfig').sumneko_lua.setup{}
            require('lspconfig').rnix.setup{}
            EOF
          '';
        }
        nvim-cmp
        cmp-nvim-lsp
        luasnip
        cmp_luasnip
        {
          plugin = edge;
          config = ''
            let g:edge_style = 'neon'
            colorscheme edge
          '';
        }
        {
          plugin = lualine-nvim;
          config = ''
            lua << EOF
            require('lualine').setup {
              options = {
                icons_enabled = false,
                theme = 'edge',
                component_separators = '|',
                section_separators = ' ',
              },
            }
            EOF
          '';
        }
        {
          plugin = indent-blankline-nvim;
          config = ''
            lua << EOF
            require('indent_blankline').setup {
              char = '┊',
              show_trailing_blankline_indent = false,
            }
            EOF
          '';
        }
        vim-sleuth
        {
          plugin = telescope-nvim;
          config = ''
            lua << EOF
            require('telescope').setup {
              defaults = {
                mappings = {
                  i = {
                    ['<C-u>'] = false,
                    ['<C-d>'] = false,
                  },
                },
              },
            }
            EOF
          '';
        }
        {
          plugin = telescope-fzf-native-nvim;
          config = "lua pcall(require('telescope').load_extension, 'fzf')";
        }
        impatient-nvim
      ];
      extraConfig = ''
        luafile ~/.config/nvim/settings.lua
      '';
      extraPackages = with pkgs; [
        rnix-lsp
        nixfmt
        rust-analyzer
        sumneko-lua-language-server
        stylua
      ];
    };
  };

  xdg.configFile = {
    "nvim/settings.lua".source = ../../modules/nvim/init.lua;
  };
}
