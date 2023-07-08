return {

  -- neovim plugin for nix
  {
    "LnL7/vim-nix",
    ft = "nix"
  },

  -- add nix treesitter
  {
    "nvim-treesitter/nvim-treesitter",
    opts = function(_, opts)
      vim.list_extend(opts.ensure_installed, { "nix" })
    end,
  },

  -- add lsp extensions to mason
  {
    "williamboman/mason.nvim",
    opts = function(_, opts)
      vim.list_extend(opts.ensure_installed, { "nil" })
    end,
  },

  -- add diagnostic and formatter options to null-ls
  {
    "jose-elias-alvarez/null-ls.nvim",
    opts = function()
      local nls = require("null-ls")
      return {
        sources = {
          nls.builtins.formatting.nixpkgs_fmt,
        },
      }
    end,
  },

  -- add lsp server for nix
  {
    "neovim/nvim-lspconfig",
    opts = {
      servers = {
        nil_ls = {},
      },
    },
  },
}
