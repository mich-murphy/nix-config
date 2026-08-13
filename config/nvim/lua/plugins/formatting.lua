return {
	{
		"stevearc/conform.nvim",
		cmd = "ConformInfo",
		event = { "BufWritePre" },
		keys = {
			{
				"<leader>cf",
				function()
					require("conform").format({ async = true, lsp_format = "fallback" })
				end,
				mode = { "n", "x" },
				desc = "Format",
			},
		},
		opts = {
			default_format_opts = {
				timeout_ms = 3000,
				lsp_format = "fallback",
			},
			format_on_save = function(buf)
				local buffer_setting = vim.b[buf].autoformat
				if buffer_setting == false or (buffer_setting == nil and vim.g.autoformat == false) then
					return nil
				end
				return { timeout_ms = 3000, lsp_format = "fallback" }
			end,
			formatters_by_ft = {
				go = { "goimports", "gofumpt" },
				javascript = { "prettier" },
				javascriptreact = { "prettier" },
				json = { "prettier" },
				jsonc = { "prettier" },
				lua = { "stylua" },
				markdown = { "prettier", "markdownlint-cli2", "markdown-toc" },
				["markdown.mdx"] = { "prettier", "markdownlint-cli2", "markdown-toc" },
				nix = { "alejandra" },
				python = { "ruff_format" },
				sh = { "shfmt" },
				typescript = { "prettier" },
				typescriptreact = { "prettier" },
				yaml = { "prettier" },
			},
			formatters = {
				["markdown-toc"] = {
					condition = function(_, context)
						for _, line in ipairs(vim.api.nvim_buf_get_lines(context.buf, 0, -1, false)) do
							if line:find("<!%-%- toc %-%->") then
								return true
							end
						end
						return false
					end,
				},
				["markdownlint-cli2"] = {
					condition = function(_, context)
						for _, diagnostic in ipairs(vim.diagnostic.get(context.buf)) do
							if diagnostic.source == "markdownlint" then
								return true
							end
						end
						return false
					end,
				},
			},
		},
	},
}
