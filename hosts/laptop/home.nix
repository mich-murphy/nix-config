{ config, pkgs, user, gitUser, gitEmail, ... }:

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
      ranger
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
    kitty = {
      enable = true;
      # fake package - managed by homebrew instead
      package = pkgs.runCommand "kitty.0.0" {} "mkdir $out";
      darwinLaunchOptions = [ "--single-instance" ];
      # kitty +kitten themes
      theme = "Tokyo Night";
      font = {
        name = "JetBrainsMono Nerd Font";
        size = 13;
      };
      settings = {
        disable_ligatures = "never";
        enable_audio_bell = "no";
        confirm_os_window_close = 0;
        window_padding_width = 10;
        scrollback_lines = 1000;
        hide_window_decorations = "titlebar-only";
        tab_bar_edge = "top";
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
            require('nvim-treesitter.configs').setup {
              highlight = {
                enable = true,
                additional_vim_regex_highlighting = false,
              },
              indent = {
                enable = true
              },
              incremental_selection = {
                enable = true,
                keymaps = {
                  init_selection = '<c-space>',
                  node_incremental = '<c-space>',
                  -- TODO: I'm not sure for this one.
                  scope_incremental = '<c-s>',
                  node_decremental = '<c-backspace>',
                },
              },
            }
            EOF
          '';
        }
        {
          # allows manipulation of additional objects e.g. paragraphs
          plugin = nvim-treesitter-textobjects;
          config = ''
            lua << EOF
            require'nvim-treesitter.configs'.setup {
              textobjects = {
                select = {
                  enable = true,
                  -- Automatically jump forward to textobj, similar to targets.vim
                  lookahead = true,
                  keymaps = {
                    -- You can use the capture groups defined in textobjects.scm
                    ["af"] = "@function.outer",
                    ["if"] = "@function.inner",
                    ["ac"] = "@class.outer",
                    ["ic"] = { query = "@class.inner", desc = "Select inner part of a class region" },
                  },
                },
                move = {
                  enable = true,
                  set_jumps = true, -- whether to set jumps in the jumplist
                  goto_next_start = {
                    [']m'] = '@function.outer',
                    [']]'] = '@class.outer',
                  },
                  goto_next_end = {
                    [']M'] = '@function.outer',
                    [']['] = '@class.outer',
                  },
                  goto_previous_start = {
                    ['[m'] = '@function.outer',
                    ['[['] = '@class.outer',
                  },
                  goto_previous_end = {
                    ['[M'] = '@function.outer',
                    ['[]'] = '@class.outer',
                  },
                },
                swap = {
                  enable = true,
                  swap_next = {
                    ['<leader>a'] = '@parameter.inner',
                  },
                  swap_previous = {
                    ['<leader>A'] = '@parameter.inner',
                  },
                },
              },
            }
            EOF
          '';
        }
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
        {
          plugin = nvim-cmp;
          config = ''
            lua << EOF
            local cmp = require 'cmp'
            local luasnip = require 'luasnip'
            
            cmp.setup {
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
                    nvim_lsp = 'λ',
                    luasnip = '⋗',
                    buffer = 'Ω',
                    path = '🖫',
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
            local servers = { 'rust-analyzer', 'sumneko_lua', 'rnix' }
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
            require('lualine').setup {
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
                prompt_prefix = " ",
                selection_caret = " ",
                path_display = { "smart" },
                mappings = {
                  i = {
                    ["<C-n>"] = actions.cycle_history_next,
                    ["<C-p>"] = actions.cycle_history_prev,
                    ["<C-j>"] = actions.move_selection_next,
                    ["<C-k>"] = actions.move_selection_previous,
                    ["<C-c>"] = actions.close,
                    ["<Down>"] = actions.move_selection_next,
                    ["<Up>"] = actions.move_selection_previous,
                    ["<CR>"] = actions.select_default,
                    ["<C-x>"] = actions.select_horizontal,
                    ["<C-v>"] = actions.select_vertical,
                    ["<C-t>"] = actions.select_tab,
                    ["<C-u>"] = actions.preview_scrolling_up,
                    ["<C-d>"] = actions.preview_scrolling_down,
                    ["<PageUp>"] = actions.results_scrolling_up,
                    ["<PageDown>"] = actions.results_scrolling_down,
                    ["<Tab>"] = actions.toggle_selection + actions.move_selection_worse,
                    ["<S-Tab>"] = actions.toggle_selection + actions.move_selection_better,
                    ["<C-q>"] = actions.send_to_qflist + actions.open_qflist,
                    ["<M-q>"] = actions.send_selected_to_qflist + actions.open_qflist,
                    ["<C-l>"] = actions.complete_tag,
                  },
                  n = {
                    ["<esc>"] = actions.close,
                    ["<CR>"] = actions.select_default,
                    ["<C-x>"] = actions.select_horizontal,
                    ["<C-v>"] = actions.select_vertical,
                    ["<C-t>"] = actions.select_tab,
                    ["<Tab>"] = actions.toggle_selection + actions.move_selection_worse,
                    ["<S-Tab>"] = actions.toggle_selection + actions.move_selection_better,
                    ["<C-q>"] = actions.send_to_qflist + actions.open_qflist,
                    ["<M-q>"] = actions.send_selected_to_qflist + actions.open_qflist,
                    ["j"] = actions.move_selection_next,
                    ["k"] = actions.move_selection_previous,
                    ["H"] = actions.move_to_top,
                    ["M"] = actions.move_to_middle,
                    ["L"] = actions.move_to_bottom,
                    ["<Down>"] = actions.move_selection_next,
                    ["<Up>"] = actions.move_selection_previous,
                    ["gg"] = actions.move_to_top,
                    ["G"] = actions.move_to_bottom,
                    ["<C-u>"] = actions.preview_scrolling_up,
                    ["<C-d>"] = actions.preview_scrolling_down,
                    ["<PageUp>"] = actions.results_scrolling_up,
                    ["<PageDown>"] = actions.results_scrolling_down,
                  },
                }
              },
              pickers = {
                -- Default configuration for builtin pickers goes here:
                -- picker_name = {
                --   picker_config_key = value,
                --   ...
                -- }
                -- Now the picker_config_key will be applied every time you call this
                -- builtin picker
              },
              extensions = {
                -- Your extension configuration goes here:
                -- extension_name = {
                --   extension_config_key = value,
                -- }
                -- please take a look at the readme of the extension you want to configure
              }
            }
            EOF
          '';
        }
        {
          plugin = telescope-fzf-native-nvim;
          config = "lua pcall(require('telescope').load_extension, 'fzf')";
        }
        {
          plugin = impatient-nvim;
          config = "lua require('impatient')";
        }
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
