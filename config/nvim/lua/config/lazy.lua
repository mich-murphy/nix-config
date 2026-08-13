local lazypath = vim.fn.stdpath("data") .. "/lazy/lazy.nvim"
local source = debug.getinfo(1, "S").source:sub(2)
local config_root = vim.fs.dirname(vim.fs.dirname(vim.fs.dirname(source)))

if not vim.uv.fs_stat(lazypath) then
	local output = vim.fn.system({
		"git",
		"clone",
		"--filter=blob:none",
		"--branch=stable",
		"https://github.com/folke/lazy.nvim.git",
		lazypath,
	})

	if vim.v.shell_error ~= 0 then
		error("Failed to install lazy.nvim:\n" .. output)
	end
end

vim.opt.rtp:prepend(lazypath)

require("lazy").setup({
	spec = { { import = "plugins" } },
	lockfile = config_root .. "/lazy-lock.json",
	local_spec = false,
	defaults = {
		lazy = true,
		version = false,
	},
	install = { colorscheme = { "tokyonight-night", "habamax" } },
	pkg = { enabled = false },
	checker = { enabled = false },
	change_detection = { enabled = false },
	rocks = { enabled = false },
	performance = {
		rtp = {
			disabled_plugins = {
				"gzip",
				"netrwPlugin",
				"tarPlugin",
				"tohtml",
				"tutor",
				"zipPlugin",
			},
		},
	},
})
