-- Options are automatically loaded before lazy.nvim startup
-- Default options that are always set: https://github.com/LazyVim/LazyVim/blob/main/lua/lazyvim/config/options.lua
-- Add any additional options here

vim.opt.scrolloff = 8
vim.opt.sidescrolloff = 8
vim.cmd[[ set grepprg=rg\ --vimgrep ]]
vim.cmd[[ set grepformat^=%f:%l:%c:%m ]]
