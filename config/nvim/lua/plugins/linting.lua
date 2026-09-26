-- golangci-lint reads the saved file and checks the whole package, so running
-- it on InsertLeave is slow and reports stale results for unsaved edits.
local skip_on_insert_leave = { golangcilint = true }

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
				callback = function(event)
					vim.defer_fn(function()
						if event.event ~= "InsertLeave" then
							pcall(lint.try_lint)
							return
						end
						if not vim.api.nvim_buf_is_valid(event.buf) then
							return
						end
						local names = vim.tbl_filter(function(name)
							return not skip_on_insert_leave[name]
						end, lint.linters_by_ft[vim.bo[event.buf].filetype] or {})
						if #names > 0 then
							pcall(lint.try_lint, names)
						end
					end, 100)
				end,
			})
		end,
	},
}
