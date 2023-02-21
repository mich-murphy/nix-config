return {
  {
    "neovim/nvim-lspconfig",
    opts = function(_, opts)
      opts.autoformat = false
    end,
  },

  -- language specific extension modules
  { import = "plugins.extras.lang.python" },
  { import = "plugins.extras.lang.nix" },
}
