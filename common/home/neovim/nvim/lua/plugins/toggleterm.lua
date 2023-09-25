return {

  -- edit toggleterm to add border
  {
    'akinsho/toggleterm.nvim',
    keys = {
        { "<leader>ft", "<cmd>exe v:count1 . 'ToggleTerm'<cr>", desc = "Terminal (root dir)", },
        { "<leader>fT", "<cmd>exe v:count1 . 'ToggleTerm dir=.'<cr>", desc = "Terminal (cwd)", },
        { "<esc>", "<C-\\><C-n>", mode = "t", },
        { "<C-h>", "<cmd>wincmd h<cr>", mode = "t", },
        { "<C-j>", "<cmd>wincmd j<cr>", mode = "t", },
        { "<C-k>", "<cmd>wincmd k<cr>", mode = "t", },
        { "<C-l>", "<cmd>wincmd l<cr>", mode = "t", },
      },
    opts = {
      size = 20,
      start_in_insert = true,
      -- direction = 'float',
      direction = 'horizontal',
      highlights = {
        Normal = {
          guibg = "#1f2335",
        },
        FloatNormal = {
          link = 'Normal',
        },
        FloatBorder = {
          guifg = "#41a6b5"
        }
      },
      float_opts = {
        border = 'rounded',
        width = 180,
        height = 45,
        winblend = 2,
      },
    }
  }
}
