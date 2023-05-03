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

  -- enable lsp_lines for diagnostics
  {
    "https://git.sr.ht/~whynothugo/lsp_lines.nvim",
    event = { "BufReadPost", "BufNewFile" },
    config = true
  },

  --disable virtual_text in place of lsp_lines
  {
    "neovim/nvim-lspconfig",
    opts = {
      diagnostics = {
        virtual_text = false
      }
    }
  }
}
