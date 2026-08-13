local servers = {
	"docker_compose_language_service",
	"dockerls",
	"gopls",
	"jsonls",
	"lua_ls",
	"marksman",
	"nixd",
	"pyright",
	"ruff",
	"rust_analyzer",
	"taplo",
	"texlab",
	"yamlls",
}

return {
	{
		"neovim/nvim-lspconfig",
		event = { "BufReadPre", "BufNewFile" },
		dependencies = { "saghen/blink.cmp" },
		config = function()
			local capabilities = require("blink.cmp").get_lsp_capabilities()
			capabilities.workspace = capabilities.workspace or {}
			capabilities.workspace.fileOperations = {
				didRename = true,
				willRename = true,
			}

			vim.lsp.config("*", { capabilities = capabilities })
			vim.lsp.enable(servers)
		end,
	},
}
