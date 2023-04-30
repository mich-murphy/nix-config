return {

  -- language specific extension modules
  { import = "plugins.extras.lang.python" },
  { import = "plugins.extras.lang.nix" },

  -- disable autoformat
  {
    "neovim/nvim-lspconfig",
    opts = function(_, opts)
      opts.autoformat = false
    end,
  },

  {
    "https://git.sr.ht/~whynothugo/lsp_lines.nvim",
    config = function()
      require("lsp_lines").setup()
    end,
  },

  -- Disable virtual_text since it's redundant due to lsp_lines.
  vim.diagnostic.config({
    virtual_text = false,
  })
}
