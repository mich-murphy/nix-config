return {

  -- enable monkaiipro
  {
    "https://gitlab.com/__tpb/monokai-pro.nvim",
    lazy = false,
    priority = 1000,
    config = function()
      vim.g.monokaipro_filter = "spectrum"
      vim.g.monokaipro_italic_comments = true
      vim.g.monokaipro_flat_float = false
      vim.g.monokaipro_flat_term = false
      vim.cmd([[colorscheme monokaipro]])
    end,
  },
}
