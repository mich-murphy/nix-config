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
      if type(opts.ensure_installed) == "table" then
        vim.list_extend(opts.ensure_installed, { "nix" })
      end
    end,
  },

  -- add lsp extensions to mason
  {
    "williamboman/mason.nvim",
    opts = function(_, opts)
      if type(opts.ensure_installed) == "table" then
        vim.list_extend(opts.ensure_installed, { "nil" })
      end
    end,
  },

  -- add diagnostic and formatter options to null-ls
  {
    "jose-elias-alvarez/null-ls.nvim",
    opts = function(_, opts)
      local nls = require("null-ls")
      table.insert(opts.sources, nls.builtins.formatting.nixpkgs_fmt)
    end,
  },

  -- add lsp server for nix
  {
    "neovim/nvim-lspconfig",
    opts = {
      servers = {
        nixd = {},
      },
    },
  },
}
