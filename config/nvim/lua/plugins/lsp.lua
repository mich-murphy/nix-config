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
		event = "FileType",
		config = function()
			-- Static copy of blink.cmp's get_lsp_capabilities() so we don't
			-- load blink.cmp just to advertise completion capabilities.
			local capabilities = vim.tbl_deep_extend("force", vim.lsp.protocol.make_client_capabilities(), {
				textDocument = {
					completion = {
						completionItem = {
							snippetSupport = true,
							commitCharactersSupport = false,
							documentationFormat = { "markdown", "plaintext" },
							deprecatedSupport = true,
							preselectSupport = false,
							tagSupport = { valueSet = { 1 } },
							insertReplaceSupport = true,
							resolveSupport = {
								properties = { "documentation", "detail", "additionalTextEdits", "command", "data" },
							},
							insertTextModeSupport = { valueSet = { 1 } },
							labelDetailsSupport = true,
						},
						completionList = {
							itemDefaults = { "commitCharacters", "editRange", "insertTextFormat", "insertTextMode", "data" },
						},
						contextSupport = true,
						insertTextMode = 1,
					},
				},
			})
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
