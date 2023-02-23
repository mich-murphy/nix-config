return {

  -- neovim plugin for nix
  { "LnL7/vim-nix" },

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
      vim.list_extend(opts.ensure_installed, { "rnix-lsp" })
    end,
  },

  -- add lsp server for nix
  {
    "neovim/nvim-lspconfig",
    opts = {
      servers = {
        rnix = {},
      },
    },
  },
}
