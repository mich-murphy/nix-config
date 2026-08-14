local parsers = {
	"bash",
	"c",
	"css",
	"diff",
	"dockerfile",
	"go",
	"gomod",
	"gosum",
	"gowork",
	"html",
	"http",
	"javascript",
	"jsdoc",
	"json",
	"json5",
	"jsonc",
	"lua",
	"luadoc",
	"luap",
	"markdown",
	"markdown_inline",
	"nix",
	"printf",
	"python",
	"query",
	"regex",
	"rust",
	"scss",
	"svelte",
	"toml",
	"tsx",
	"typst",
	"typescript",
	"vim",
	"vimdoc",
	"vue",
	"xml",
	"yaml",
}

local textobject_moves = {
	goto_next_start = { ["]f"] = "@function.outer", ["]c"] = "@class.outer", ["]a"] = "@parameter.inner" },
	goto_next_end = { ["]F"] = "@function.outer", ["]C"] = "@class.outer", ["]A"] = "@parameter.inner" },
	goto_previous_start = { ["[f"] = "@function.outer", ["[c"] = "@class.outer", ["[a"] = "@parameter.inner" },
	goto_previous_end = { ["[F"] = "@function.outer", ["[C"] = "@class.outer", ["[A"] = "@parameter.inner" },
}

return {
	{
		"nvim-treesitter/nvim-treesitter",
		branch = "main",
		lazy = false,
		build = ":TSUpdate",
		config = function()
			local treesitter = require("nvim-treesitter")
			treesitter.setup()

			local installed = {}
			for _, parser in ipairs(treesitter.get_installed()) do
				installed[parser] = true
			end
			local missing = vim.tbl_filter(function(parser)
				return not installed[parser]
			end, parsers)
			if #missing > 0 and vim.env.NVIM_SKIP_BOOTSTRAP ~= "1" then
				treesitter.install(missing, { summary = true })
			end

			vim.api.nvim_create_autocmd("FileType", {
				group = vim.api.nvim_create_augroup("nvim_treesitter", { clear = true }),
				callback = function(event)
					local language = vim.treesitter.language.get_lang(event.match)
					if not language or not vim.tbl_contains(treesitter.get_installed(), language) then
						return
					end

					local function has_query(name)
						local ok, query = pcall(vim.treesitter.query.get, language, name)
						return ok and query ~= nil
					end

					if has_query("highlights") then
						pcall(vim.treesitter.start, event.buf, language)
					end
					if has_query("indents") then
						vim.bo[event.buf].indentexpr = "v:lua.require'nvim-treesitter'.indentexpr()"
					end
					if has_query("folds") then
						for _, win in ipairs(vim.fn.win_findbuf(event.buf)) do
							vim.wo[win].foldmethod = "expr"
							vim.wo[win].foldexpr = "v:lua.vim.treesitter.foldexpr()"
						end
					end
				end,
			})
		end,
	},

	{
		"nvim-treesitter/nvim-treesitter-context",
		event = { "BufReadPost", "BufNewFile" },
		opts = {
			max_lines = 3,
			mode = "cursor",
		},
		keys = {
			{ "<leader>uC", "<cmd>TSContext toggle<cr>", desc = "Treesitter context" },
		},
	},

	{
		"nvim-treesitter/nvim-treesitter-textobjects",
		branch = "main",
		event = "VeryLazy",
		opts = {
			move = {
				enable = true,
				set_jumps = true,
			},
		},
		config = function(_, opts)
			require("nvim-treesitter-textobjects").setup(opts)

			local function attach(buf)
				local language = vim.treesitter.language.get_lang(vim.bo[buf].filetype)
				local ok, query = pcall(vim.treesitter.query.get, language, "textobjects")
				if not ok or not query then
					return
				end

				for method, mappings in pairs(textobject_moves) do
					for lhs, capture in pairs(mappings) do
						local move_method = method
						local move_lhs = lhs
						local move_capture = capture
						local object = capture:gsub("@", ""):gsub("%..*", "")
						local position = lhs:sub(2, 2) == lhs:sub(2, 2):upper() and "end" or "start"
						local direction = lhs:sub(1, 1) == "[" and "Previous" or "Next"
						vim.keymap.set({ "n", "x", "o" }, lhs, function()
							if vim.wo.diff and move_lhs:find("[cC]") then
								vim.cmd.normal({ move_lhs, bang = true })
								return
							end
							require("nvim-treesitter-textobjects.move")[move_method](move_capture, "textobjects")
						end, {
							buffer = buf,
							desc = ("%s %s %s"):format(direction, object, position),
							silent = true,
						})
					end
				end
			end

			vim.api.nvim_create_autocmd("FileType", {
				group = vim.api.nvim_create_augroup("nvim_treesitter_textobjects", { clear = true }),
				callback = function(event)
					attach(event.buf)
				end,
			})
			for _, buf in ipairs(vim.api.nvim_list_bufs()) do
				if vim.api.nvim_buf_is_loaded(buf) then
					attach(buf)
				end
			end
		end,
	},

	{
		"nvim-mini/mini.ai",
		event = "VeryLazy",
		opts = function()
			local ai = require("mini.ai")
			return {
				n_lines = 500,
				custom_textobjects = {
					o = ai.gen_spec.treesitter({
						a = { "@block.outer", "@conditional.outer", "@loop.outer" },
						i = { "@block.inner", "@conditional.inner", "@loop.inner" },
					}),
					f = ai.gen_spec.treesitter({ a = "@function.outer", i = "@function.inner" }),
					c = ai.gen_spec.treesitter({ a = "@class.outer", i = "@class.inner" }),
					t = { "<([%p%w]-)%f[^<%w][^<>]->.-</%1>", "^<.->().*()</[^/]->$" },
					d = { "%f[%d]%d+" },
					u = ai.gen_spec.function_call(),
					U = ai.gen_spec.function_call({ name_pattern = "[%w_]" }),
				},
			}
		end,
	},

	{
		"nvim-mini/mini.pairs",
		event = "VeryLazy",
		opts = {},
	},

	{
		"folke/lazydev.nvim",
		ft = "lua",
		cmd = "LazyDev",
		opts = {
			library = {
				{ path = "${3rd}/luv/library", words = { "vim%.uv" } },
				{ path = "snacks.nvim", words = { "Snacks" } },
				{ path = "lazy.nvim", words = { "lazy" } },
			},
		},
	},

	{
		"saghen/blink.cmp",
		version = "1.*",
		event = { "InsertEnter", "CmdlineEnter", "VeryLazy" },
		dependencies = { "rafamadriz/friendly-snippets" },
		opts = {
			snippets = { preset = "default" },
			keymap = {
				preset = "enter",
				["<C-y>"] = { "select_and_accept" },
			},
			completion = {
				accept = { auto_brackets = { enabled = true } },
				documentation = { auto_show = true, auto_show_delay_ms = 200 },
				menu = { draw = { treesitter = { "lsp" } } },
			},
			sources = {
				default = { "lsp", "path", "snippets", "buffer" },
				per_filetype = { lua = { inherit_defaults = true, "lazydev" } },
				providers = {
					lazydev = {
						name = "LazyDev",
						module = "lazydev.integrations.blink",
						score_offset = 100,
					},
				},
			},
			cmdline = {
				enabled = true,
				keymap = {
					preset = "cmdline",
					["<Right>"] = false,
					["<Left>"] = false,
				},
				completion = {
					list = { selection = { preselect = false } },
					menu = {
						auto_show = function()
							return vim.fn.getcmdtype() == ":"
						end,
					},
					ghost_text = { enabled = true },
				},
			},
		},
	},
}
