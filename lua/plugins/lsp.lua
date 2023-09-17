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
  },

  -- add symbols-outline
  {
    "simrat39/symbols-outline.nvim",
    cmd = "SymbolsOutline",
    keys = { { "<leader>cs", "<cmd>SymbolsOutline<cr>", desc = "Symbols Outline" } },
    opts = {
      -- add your options that should be passed to the setup() function here
      position = "right",
    },
  },
}
