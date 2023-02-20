-- Keymaps are automatically loaded on the VeryLazy event
-- Default keymaps that are always set: https://github.com/LazyVim/LazyVim/blob/main/lua/lazyvim/config/keymaps.lua
-- Add any additional keymaps here
-- DAP
vim.keymap.set("n", "<F1>", ":lua require'dap'.step_into()<CR>")
vim.keymap.set("n", "<F2>", ":lua require'dap'.step_out()<CR>")
vim.keymap.set("n", "<F3>", ":lua require'dap'.step_over()<CR>")
vim.keymap.set("n", "<F4>", ":lua require'dap'.step_continue()<CR>")
vim.keymap.set("n", "<leader>db", ":lua require'dab'.toggle_breakpoint()<CR>")
vim.keymap.set("n", "<leader>dB", ":lua require'dab'.set_breakpoint(vim.fn.input('Breakpoint condition: '))<CR>")
