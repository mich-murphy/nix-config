{ config, pkgs, user, gitUser, gitEmail, ... }:

let 
  neovim-nightly = pkgs.neovim-unwrapped.overrideAttrs (old: rec {
    version = "0.8.0-dev";
    src = old.fetchFromGitHub {
      owner = "neovim";
      repo = "neovim";
      rev = "v${version}";
      sha256 = "Hx8y6wzot/IvtZdYsERJiLWjW6u11tUyiA2PK90hGD4=";
    };
    buildInputs = old.buildInputs ++ [ pkgs.tree-sitter ];
  });
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
        ssh = "kitty +kitten ssh";
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
            background = "0x1a1b26";
            foreground = "0xc0caf5";
          };
          normal = {
            black = "0x15161e";
            red = "0xf7768e";
            green = "0x9ece6a";
            yellow = "0xe0af68";
            blue = "0x7aa2f7";
            magenta = "0xbb9af7";
            cyan = "0x7dcfff";
            white = "0xa9b1d6";
          }; 
          bright = {
            black = "0x414868";
            red = "0xf7768e";
            green = "0x9ece6a";
            yellow = "0xe0af68";
            blue = "0x7aa2f7";
            magenta = "0xbb9af7";
            cyan = "0x7dcfff";
            white = "0xc0caf5";
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
    neovim = {
      enable = true;
      package = pkgs.neovim-unwrapped;
      viAlias = true;
      vimAlias = true;
      withPython3 = true;
      plugins = with pkgs.vimPlugins; [ 
        vim-surround
        vim-fugitive
        vim-nix
        {
          plugin = comment-nvim;
          config = "lua require('Comment').setup{}";
        }
        {
          plugin = gitsigns-nvim;
          config = ''
            lua << EOF
            require('gitsigns').setup{
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
          # install treesitter and listed grammars
          plugin = (
            nvim-treesitter.withPlugins (plugins: [
              plugins.tree-sitter-python
              plugins.tree-sitter-lua
              plugins.tree-sitter-nix
              plugins.tree-sitter-yaml
              plugins.tree-sitter-toml
              plugins.tree-sitter-bash
              plugins.tree-sitter-vim
              plugins.tree-sitter-markdown
              plugins.tree-sitter-json
              plugins.tree-sitter-html
              plugins.tree-sitter-fish
              plugins.tree-sitter-dockerfile
              plugins.tree-sitter-css
            ])
          );
          config = ''
            lua << EOF
            require('nvim-treesitter.configs').setup{
              highlight = {
                enable = true,
                additional_vim_regex_highlighting = false,
              },
              indent = {
                enable = true,
              },
              autopairs = {
                enable = true,
              },
            }
            EOF
          '';
        }
        {
          plugin = nvim-treesitter-context;
          config = ''
            lua << EOF
            require'treesitter-context'.setup{
              enable = true, -- Enable this plugin (Can be enabled/disabled later via commands)
              max_lines = 0, -- How many lines the window should span. Values <= 0 mean no limit.
              trim_scope = 'outer', -- Which context lines to discard if `max_lines` is exceeded. Choices: 'inner', 'outer'
              patterns = { -- Match patterns for TS nodes. These get wrapped to match at word boundaries.
                default = {
                    'class',
                    'function',
                    'method',
                    'for',
                    'while',
                    'if',
                    'switch',
                    'case',
                },
                tex = {
                    'chapter',
                    'section',
                    'subsection',
                    'subsubsection',
                },
                rust = {
                    'impl_item',
                    'struct',
                    'enum',
                },
                scala = {
                    'object_definition',
                },
                vhdl = {
                    'process_statement',
                    'architecture_body',
                    'entity_declaration',
                },
                markdown = {
                    'section',
                },
                elixir = {
                    'anonymous_function',
                    'arguments',
                    'block',
                    'do_block',
                    'list',
                    'map',
                    'tuple',
                    'quoted_content',
                },
                json = {
                    'pair',
                },
                yaml = {
                    'block_mapping_pair',
                },
              },
              exact_patterns = {
              },
              zindex = 20, -- The Z-index of the context window
              mode = 'cursor',  -- Line used to calculate context. Choices: 'cursor', 'topline'
              separator = nil,
            }
            EOF
          '';
        } 
        {
          plugin = nvim-autopairs;
          config = ''
            lua << EOF
            require('nvim-autopairs').setup{
              check_ts = true,
              ts_config = {
                lua = { "string", "source" },
                javascript = { "string", "template_string" },
                java = false,
              },
              disable_filetype = { "TelescopePrompt", "spectre_panel" },
              fast_wrap = {
                map = "<M-e>",
                chars = { "{", "[", "(", '"', "'" },
                pattern = string.gsub([[ [%'%"%)%>%]%)%}%,] ]], "%s+", ""),
                offset = 0, -- Offset from pattern match
                end_key = "$",
                keys = "qwertyuiopzxcvbnmasdfghjkl",
                check_comma = true,
                highlight = "PmenuSel",
                highlight_grey = "LineNr",
              },
            }
            EOF
          '';
        }
        {
          plugin = nvim-lspconfig;
          config = ''
            lua << EOF
            require('lspconfig').sumneko_lua.setup{}
            require('lspconfig').rnix.setup{}
            require('lspconfig').pyright.setup{}
            EOF
          '';
        }
        {
          plugin = nvim-cmp;
          config = ''
            lua << EOF
            local cmp = require 'cmp'
            local luasnip = require 'luasnip'
            cmp.setup{
              snippet = {
                expand = function(args)
                  luasnip.lsp_expand(args.body)
                end,
              },
              mapping = cmp.mapping.preset.insert {
                ['<C-d>'] = cmp.mapping.scroll_docs(-4),
                ['<C-f>'] = cmp.mapping.scroll_docs(4),
                ['<C-Space>'] = cmp.mapping.complete(),
                ['<CR>'] = cmp.mapping.confirm {
                  behavior = cmp.ConfirmBehavior.Replace,
                  select = true,
                },
                ['<Tab>'] = cmp.mapping(function(fallback)
                  if cmp.visible() then
                    cmp.select_next_item()
                  elseif luasnip.expand_or_jumpable() then
                    luasnip.expand_or_jump()
                  else
                    fallback()
                  end
                end, { 'i', 's' }),
                ['<S-Tab>'] = cmp.mapping(function(fallback)
                  if cmp.visible() then
                    cmp.select_prev_item()
                  elseif luasnip.jumpable(-1) then
                    luasnip.jump(-1)
                  else
                    fallback()
                  end
                end, { 'i', 's' }),
              },
              sources = {
                { name = 'path' },
                { name = 'nvim_lsp' },
                { name = 'buffer' },
                { name = 'luasnip' },
              },
              formatting = {
                fields = {'menu', 'abbr', 'kind'},
                format = function(entry, item)
                  local menu_icon = {
                    nvim_lsp = '',
                    luasnip = '',
                    buffer = '﬘',
                    path = '',
                  }
                  item.menu = menu_icon[entry.source.name]
                  return item
                end,
              },
            }
            EOF
          '';
        }
        {
          plugin = cmp-nvim-lsp;
          config = ''
            lua << EOF
            local capabilities = vim.lsp.protocol.make_client_capabilities()
            capabilities = require('cmp_nvim_lsp').update_capabilities(capabilities)
            local servers = { 
              'sumneko_lua', 
              'rnix', 
              'pyright' 
            }
            for _, lsp in ipairs(servers) do
              require('lspconfig')[lsp].setup { 
                capabilities = capabilities,
              }
            end
            EOF
          '';
        }
        luasnip
        cmp_luasnip
        cmp-path
        cmp-buffer
        {
          plugin = tokyonight-nvim;
          config = ''
            colorscheme tokyonight-night
          '';
        }
        {
          plugin = lualine-nvim;
          config = ''
            lua << EOF
            require('lualine').setup{
              options = {
                icons_enabled = false,
                theme = 'tokyonight',
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
            require('indent_blankline').setup{
              char = '┊',
              show_trailing_blankline_indent = false,
            }
            EOF
          '';
        }
        vim-sleuth
        {
          plugin = null-ls-nvim;
          config = ''
            lua << EOF
            require("null-ls").setup({
              sources = {
                require("null-ls").builtins.formatting.stylua,
                require("null-ls").builtins.formatting.black,
                require("null-ls").builtins.diagnostics.eslint,
                require("null-ls").builtins.diagnostics.flake8,
                require("null-ls").builtins.completion.spell,
              },
            })
            EOF
          '';
        }
        {
          plugin = telescope-nvim;
          config = "lua require('telescope').setup{}";
        }
        {
          plugin = telescope-fzf-native-nvim;
          config = "lua pcall(require('telescope').load_extension, 'fzf')";
        }
      ];
      extraConfig = ''
        luafile ~/.config/nvim/settings.lua
      '';
      extraPackages = with pkgs; [
        rnix-lsp
        nixfmt
        sumneko-lua-language-server
        stylua
        nodePackages.pyright
      ];
      extraPython3Packages = (ps: with ps; [
        pkgs.python310Packages.flake8
        pkgs.python310Packages.black
      ]);
    };
  };

  xdg.configFile = {
    "nvim/settings.lua".source = ../../modules/nvim/init.lua;
    "ranger/rc.conf".source = ../../modules/ranger/rc.conf;
  };
}
