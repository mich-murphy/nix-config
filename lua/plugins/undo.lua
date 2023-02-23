return {

  { "simnalamburt/vim-mundo" },

  -- Set keymap to toggle plugin
  vim.keymap.set("n", "<leader>su", "<cmd>MundoToggle<cr>", { desc = "Search undo tree" })
}
