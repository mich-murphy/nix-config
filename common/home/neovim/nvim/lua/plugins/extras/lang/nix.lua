return {
  { "LnL7/vim-nix" },
  -- add python treesitter
  {
    "nvim-treesitter/nvim-treesitter",
    opts = function(_, opts)
      vim.list_extend(opts.ensure_installed, { "nix" })
    end,
  },
}
