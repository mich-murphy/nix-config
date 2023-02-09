{ lib, config, pkgs, ... }:

{
  imports = [
    ./lsp.nix
    ./treesitter.nix
    ./ui.nix
    ./telescope.nix
  ];

  programs.neovim = {
    enable = true;
    viAlias = true;
    vimAlias = true;
    withPython3 = true;
    plugins = with pkgs.vimPlugins; [
      vim-surround
      vim-fugitive
      vim-bbye
      vim-nix
      direnv-vim
      {
        plugin = nvim-autopairs;
        type = "lua";
        config = ''
          local status_ok, npairs = pcall(require, "nvim-autopairs")
          if not status_ok then
            return
          end

          npairs.setup {
            check_ts = true, -- treesitter integration
            ts_config = {
              lua = { "string", "source" },
              javascript = { "string", "template_string" },
              java = false,
            },
            disable_filetype = { "TelescopePrompt" },
            fast_wrap = {
              map = "<M-e>",
              chars = { "{", "[", "(", '"', "'" },
              pattern = string.gsub([[ [%'%"%)%>%]%)%}%,] ]], "%s+", ""),
              offset = 0,
              end_key = "$",
              keys = "qwertyuiopzxcvbnmasdfghjkl",
              check_comma = true,
              highlight = "PmenuSel",
              highlight_grey = "LineNr",
            },
          }

          local cmp_autopairs = require "nvim-autopairs.completion.cmp"
          local cmp_status_ok, cmp = pcall(require, "cmp")
          if not cmp_status_ok then
            return
          end
          cmp.event:on("confirm_done", cmp_autopairs.on_confirm_done {})
        '';
      }
      {
        plugin = nvim-colorizer-lua;
        type = "lua";
        config = ''
          local status_ok, colorizer = pcall(require, "colorizer")
          if not status_ok then
            return
          end

          colorizer.setup {
            filetypes = { "*" },
            user_default_options = {
              RGB = true, -- #RGB hex codes
              RRGGBB = true, -- #RRGGBB hex codes
              names = false, -- "Name" codes like Blue
              RRGGBBAA = false, -- #RRGGBBAA hex codes
              rgb_fn = false, -- CSS rgb() and rgba() functions
              hsl_fn = false, -- CSS hsl() and hsla() functions
              css = false, -- Enable all css features: rgb_fn, hsl_fn, names, RGB, RRGGBB
              css_fn = false, -- Enable all CSS *functions*: rgb_fn, hsl_fn
              mode = "background", -- Set the display mode
            },
          }
        '';
      }
      {
        plugin = comment-nvim;
        type = "lua";
        config = ''
          local status_ok, comment = pcall(require, "Comment")
          if not status_ok then
            return
          end

          comment.setup {
            pre_hook = function(ctx)
              local U = require "Comment.utils"

              local location = nil
              if ctx.ctype == U.ctype.block then
                location = require("ts_context_commentstring.utils").get_cursor_location()
              elseif ctx.cmotion == U.cmotion.v or ctx.cmotion == U.cmotion.V then
                location = require("ts_context_commentstring.utils").get_visual_start_location()
              end

              return require("ts_context_commentstring.internal").calculate_commentstring {
                key = ctx.ctype == U.ctype.line and "__default" or "__multiline",
                location = location,
              }
            end,
          }
        '';
      }
      {
        plguin = indent-blankline-nvim;
        type = "lua";
        config = ''
          local status_ok, indent_blankline = pcall(require, "indent_blankline")
          if not status_ok then
            return
          end

          indent_blankline.setup {
            char = "▏",
            show_trailing_blankline_indent = false,
            show_first_indent_level = true,
            use_treesitter = true,
            show_current_context = true,
            buftype_exclude = { 
              "terminal", 
              "nofile",
            },
            filetype_exclude = {
              "help",
              "NvimTree",
            },
            context_patterns = {
              "class",
              "return",
              "function",
              "method",
              "^if",
              "^while",
              "jsx_element",
              "^for",
              "^object",
              "^table",
              "block",
              "arguments",
              "if_statement",
              "else_clause",
              "jsx_element",
              "jsx_self_closing_element",
              "try_statement",
              "catch_clause",
              "import_statement",
              "operation_type",
            },
          }
        '';
      }
      # git
      {
        plugin = gitsigns-nvim;
        type = "lua";
        config = ''
          local status_ok, gitsigns = pcall(require, "gitsigns")
          if not status_ok then
            return
          end

          gitsigns.setup {
            signs = {
              add = { 
                hl = "GitSignsAdd", 
                text = "▎", 
                numhl = "GitSignsAddNr", 
                linehl = "GitSignsAddLn" 
              },
              change = { 
                hl = "GitSignsChange", 
                text = "▎", 
                numhl = "GitSignsChangeNr", 
                linehl = "GitSignsChangeLn" 
              },
              delete = { 
                hl = "GitSignsDelete", 
                text = "契", 
                numhl = "GitSignsDeleteNr", 
                linehl = "GitSignsDeleteLn" 
              },
              topdelete = { 
                hl = "GitSignsDelete", 
                text = "契", 
                numhl = "GitSignsDeleteNr", 
                linehl = "GitSignsDeleteLn" 
              },
              changedelete = { 
                hl = "GitSignsChange", 
                text = "▎", 
                numhl = "GitSignsChangeNr", 
                linehl = "GitSignsChangeLn" 
              },
            },
          }
        '';
      }
      # tmux
      vim-tmux-navigator
      tmux-complete-vim
      vim-tmux
    ];
    extraConfig = {
      lua = ''
        -- Options
        vim.opt.backup = false                          -- creates a backup file
        vim.opt.clipboard = "unnamedplus"               -- allows neovim to access the system clipboard
        vim.opt.cmdheight = 1                           -- more space in the neovim command line for displaying messages
        vim.opt.completeopt = { "menuone", "noselect" } -- mostly just for cmp
        vim.opt.conceallevel = 0                        -- so that `` is visible in markdown files
        vim.opt.fileencoding = "utf-8"                  -- the encoding written to a file
        vim.opt.hlsearch = true                         -- highlight all matches on previous search pattern
        vim.opt.ignorecase = true                       -- ignore case in search patterns
        vim.opt.mouse = "a"                             -- allow the mouse to be used in neovim
        vim.opt.pumheight = 10                          -- pop up menu height
        vim.opt.showmode = false                        -- we don't need to see things like -- INSERT -- anymore
        vim.opt.showtabline = 0                         -- always show tabs
        vim.opt.smartcase = true                        -- smart case
        vim.opt.smartindent = true                      -- make indenting smarter again
        vim.opt.splitbelow = true                       -- force all horizontal splits to go below current window
        vim.opt.splitright = true                       -- force all vertical splits to go to the right of current window
        vim.opt.swapfile = false                        -- creates a swapfile
        vim.opt.termguicolors = true                    -- set term gui colors (most terminals support this)
        vim.opt.timeoutlen = 1000                       -- time to wait for a mapped sequence to complete (in milliseconds)
        vim.opt.undofile = true                         -- enable persistent undo
        vim.opt.updatetime = 300                        -- faster completion (4000ms default)
        vim.opt.writebackup = false                     -- if a file is being edited by another program (or was written to file while editing with another program), it is not allowed to be edited
        vim.opt.expandtab = true                        -- convert tabs to spaces
        vim.opt.shiftwidth = 2                          -- the number of spaces inserted for each indentation
        vim.opt.tabstop = 2                             -- insert 2 spaces for a tab
        vim.opt.cursorline = true                       -- highlight the current line
        vim.opt.number = true														-- set numbered lines
        vim.opt.relativenumber = true                           
        vim.opt.laststatus = 3
        vim.opt.showcmd = false
        vim.opt.ruler = false
        vim.opt.numberwidth = 4                         -- set number column width to 2 {default 4}
        vim.opt.signcolumn = "yes"                      -- always show the sign column, otherwise it would shift the text each time
        vim.opt.wrap = false                            -- display lines as one long line
        vim.opt.scrolloff = 8                           -- is one of my fav
        vim.opt.sidescrolloff = 8
        vim.opt.guifont = "monospace:h17"               -- the font used in graphical neovim applications
        vim.opt.fillchars.eob=" "
        vim.opt.shortmess:append "c"
        vim.opt.whichwrap:append("<,>,[,],h,l")
        vim.opt.iskeyword:append("-")

        -- Keymaps
        local keymap = vim.keymap.set
        local opts = { silent = true }
        keymap("", "<Space>", "<Nop>", opts)
        vim.g.mapleader = " "

        -- Modes
        -- normal_mode = "n",
        -- insert_mode = "i",
        -- visual_mode = "v",
        -- visual_block_mode = "x",
        -- term_mode = "t",
        -- command_mode = "c",

        -- Better window navigation
        keymap("n", "<C-h>", "<C-w>h", opts)
        keymap("n", "<C-j>", "<C-w>j", opts)
        keymap("n", "<C-k>", "<C-w>k", opts)
        keymap("n", "<C-l>", "<C-w>l", opts)

        -- Resize with arrows
        keymap("n", "<A-k>", ":resize -2<CR>", opts)
        keymap("n", "<A-j>", ":resize +2<CR>", opts)
        keymap("n", "<A-l>", ":vertical resize -2<CR>", opts)
        keymap("n", "<A-h>", ":vertical resize +2<CR>", opts)

        -- Navigate buffers
        keymap("n", "<S-l>", ":bnext<CR>", opts)
        keymap("n", "<S-h>", ":bprevious<CR>", opts)

        -- Clear highlights
        keymap("n", "<leader>h", "<cmd>nohlsearch<CR>", opts)

        -- Close buffers
        keymap("n", "<S-q>", "<cmd>Bdelete!<CR>", opts)

        -- Better paste
        keymap("v", "p", '"_dP', opts)

        -- Executable and source file
        keymap("n", "<C-x>", ":! chmod +x %<CR>", opts)
        keymap("n", "<C-s>", ":source %<CR>", opts)

        -- Yank to end of line
        keymap("n", "Y", "yg$", opts)

        -- Center cursor movement for search and line concat
        keymap("n", "n", "nzzzv", opts)
        keymap("n", "N", "Nzzzv", opts)
        keymap("n", "J", "mzJ`z", opts)

        -- Stay in indent mode
        keymap("v", "<", "<gv", opts)
        keymap("v", ">", ">gv", opts)

        -- Plugins --

        -- NvimTree
        keymap("n", "<leader>e", ":NvimTreeToggle<CR>", opts)

        -- Telescope
        keymap("n", "<leader>ff", ":Telescope find_files<CR>", opts)
        keymap("n", "<leader>fg", ":Telescope live_grep<CR>", opts)
        keymap("n", "<leader>fp", ":Telescope projects<CR>", opts)
        keymap("n", "<leader>fb", ":Telescope buffers<CR>", opts)
        keymap("n", "<leader>fh", ":Telescope help_tags<CR>", opts)
        keymap("n", "<leader>fk", ":Telescope keymaps<CR>", opts)
        keymap("n", "<leader>fc", ":Telescope commands<CR>", opts)
        keymap("n", "<leader>fe", ":Telescope file_browser<CR>", opts)

        -- Comment
        keymap("n", "<leader>/", "<cmd>lua require('Comment.api').toggle.linewise.current()<CR>", opts)
        keymap("x", "<leader>/", '<ESC><CMD>lua require("Comment.api").toggle.linewise(vim.fn.visualmode())<CR>')
      '';
    };
  };
}
