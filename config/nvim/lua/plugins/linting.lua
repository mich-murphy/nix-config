return {
	{
		"mfussenegger/nvim-lint",
		event = { "BufReadPost", "BufNewFile" },
		config = function()
			local lint = require("lint")
			lint.linters_by_ft = {
				dockerfile = { "hadolint" },
				go = { "golangcilint" },
				markdown = { "markdownlint-cli2" },
				["markdown.mdx"] = { "markdownlint-cli2" },
			}

			vim.api.nvim_create_autocmd({ "BufWritePost", "BufReadPost", "InsertLeave" }, {
				group = vim.api.nvim_create_augroup("nvim_lint", { clear = true }),
				callback = function()
					vim.defer_fn(function()
						pcall(lint.try_lint)
					end, 100)
				end,
			})
		end,
	},
}
