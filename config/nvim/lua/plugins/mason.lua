local tools = {
	"bash-language-server",
	"docker-compose-language-service",
	"dockerfile-language-server",
	"eslint-lsp",
	"gofumpt",
	"goimports",
	"golangci-lint",
	"gopls",
	"hadolint",
	"json-lsp",
	"lua-language-server",
	"markdown-toc",
	"markdownlint-cli2",
	"marksman",
	"prettier",
	"pyright",
	"ruff",
	"rust-analyzer",
	"shellcheck",
	"shfmt",
	"stylua",
	"taplo",
	"texlab",
	"tinymist",
	"vtsls",
	"yaml-language-server",
}

return {
	{
		"mason-org/mason.nvim",
		cmd = "Mason",
		init = function()
			local mason_bin = vim.fs.joinpath(vim.fn.stdpath("data"), "mason", "bin")
			vim.env.PATH = mason_bin .. ":" .. vim.env.PATH
		end,
		opts = {
			PATH = "skip",
			ui = { border = "rounded" },
		},
		keys = {
			{ "<leader>cm", "<cmd>Mason<cr>", desc = "Mason" },
		},
	},

	{
		"WhoIsSethDaniel/mason-tool-installer.nvim",
		build = ":MasonToolsInstallSync",
		cmd = {
			"MasonToolsClean",
			"MasonToolsInstall",
			"MasonToolsInstallSync",
			"MasonToolsUpdate",
			"MasonToolsUpdateSync",
		},
		dependencies = { "mason-org/mason.nvim" },
		opts = {
			ensure_installed = tools,
			auto_update = false,
			run_on_start = false,
			integrations = {
				["mason-lspconfig"] = false,
				["mason-null-ls"] = false,
				["mason-nvim-dap"] = false,
			},
		},
	},
}
