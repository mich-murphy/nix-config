return {

  -- add context and previous disabling of indentation for Python
  {
    "nvim-treesitter/nvim-treesitter",
    dependencies = {
      "nvim-treesitter/nvim-treesitter-context",
    },
    opts = {
      indent = { enable = true },
    }
  },
}
