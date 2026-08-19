return {
	{
		"MagicDuck/grug-far.nvim",
		cmd = { "GrugFar", "GrugFarWithin" },
		opts = { headerMaxWidth = 80 },
		keys = {
			{
				"<leader>sr",
				function()
					local extension = vim.bo.buftype == "" and vim.fn.expand("%:e") or nil
					require("grug-far").open({
						transient = true,
						prefills = { filesFilter = extension and extension ~= "" and "*." .. extension or nil },
					})
				end,
				mode = { "n", "x" },
				desc = "Search and replace",
			},
		},
	},

	{
		"folke/flash.nvim",
		event = "VeryLazy",
		opts = {},
		keys = {
			{
				"s",
				function()
					require("flash").jump()
				end,
				mode = { "n", "x", "o" },
				desc = "Flash",
			},
			{
				"S",
				function()
					require("flash").treesitter()
				end,
				mode = { "n", "x", "o" },
				desc = "Flash Treesitter",
			},
			{
				"r",
				function()
					require("flash").remote()
				end,
				mode = "o",
				desc = "Remote Flash",
			},
			{
				"R",
				function()
					require("flash").treesitter_search()
				end,
				mode = { "o", "x" },
				desc = "Treesitter search",
			},
		},
	},

	{
		"folke/trouble.nvim",
		cmd = "Trouble",
		opts = {},
		keys = {
			{
				"<leader>xx",
				"<cmd>Trouble diagnostics toggle<cr>",
				desc = "Workspace diagnostics",
			},
			{
				"<leader>xX",
				"<cmd>Trouble diagnostics toggle filter.buf=0<cr>",
				desc = "Buffer diagnostics",
			},
			{
				"<leader>xL",
				"<cmd>Trouble loclist toggle<cr>",
				desc = "Location list",
			},
			{
				"<leader>xQ",
				"<cmd>Trouble qflist toggle<cr>",
				desc = "Quickfix list",
			},
		},
	},

	{
		"lewis6991/gitsigns.nvim",
    enabled = false,
		event = { "BufReadPre", "BufNewFile" },
		opts = {
			signs = {
				add = { text = "▎" },
				change = { text = "▎" },
				delete = { text = "" },
				topdelete = { text = "" },
				changedelete = { text = "▎" },
				untracked = { text = "▎" },
			},
			signs_staged = {
				add = { text = "▎" },
				change = { text = "▎" },
				delete = { text = "" },
				topdelete = { text = "" },
				changedelete = { text = "▎" },
			},
			on_attach = function(buf)
				local gs = require("gitsigns")
				local function map(mode, lhs, rhs, desc)
					vim.keymap.set(mode, lhs, rhs, { buffer = buf, desc = desc, silent = true })
				end
				map("n", "]h", function()
					if vim.wo.diff then
						vim.cmd.normal({ "]c", bang = true })
					else
						gs.nav_hunk("next")
					end
				end, "Next hunk")
				map("n", "[h", function()
					if vim.wo.diff then
						vim.cmd.normal({ "[c", bang = true })
					else
						gs.nav_hunk("prev")
					end
				end, "Previous hunk")
				map("n", "]H", function()
					gs.nav_hunk("last")
				end, "Last hunk")
				map("n", "[H", function()
					gs.nav_hunk("first")
				end, "First hunk")
				map({ "n", "x" }, "<leader>ghs", ":Gitsigns stage_hunk<cr>", "Stage hunk")
				map({ "n", "x" }, "<leader>ghr", ":Gitsigns reset_hunk<cr>", "Reset hunk")
				map("n", "<leader>ghS", gs.stage_buffer, "Stage buffer")
				map("n", "<leader>ghu", gs.undo_stage_hunk, "Undo stage hunk")
				map("n", "<leader>ghR", gs.reset_buffer, "Reset buffer")
				map("n", "<leader>ghp", gs.preview_hunk_inline, "Preview hunk")
				map("n", "<leader>ghb", function()
					gs.blame_line({ full = true })
				end, "Blame line")
				map("n", "<leader>ghB", gs.blame, "Blame buffer")
				map("n", "<leader>ghd", gs.diffthis, "Diff against index")
				map("n", "<leader>ghD", function()
					gs.diffthis("~")
				end, "Diff against previous revision")
				map({ "o", "x" }, "ih", ":<C-u>Gitsigns select_hunk<cr>", "Git hunk")
			end,
		},
	},

	{
		"gbprod/yanky.nvim",
		dependencies = { "folke/snacks.nvim" },
		opts = {
			system_clipboard = { sync_with_ring = not vim.env.SSH_CONNECTION },
			highlight = { timer = 150 },
		},
		keys = {
			{
				"<leader>sy",
				function()
					Snacks.picker.yanky()
				end,
				desc = "Yank history",
			},
			{ "y", "<Plug>(YankyYank)", mode = { "n", "x" }, desc = "Yank text" },
			{ "p", "<Plug>(YankyPutAfter)", desc = "Put after cursor" },
			{ "P", "<Plug>(YankyPutBefore)", desc = "Put before cursor" },
			{ "gp", "<Plug>(YankyGPutAfter)", desc = "Put after selection" },
			{ "gP", "<Plug>(YankyGPutBefore)", desc = "Put before selection" },
			{ "[y", "<Plug>(YankyCycleForward)", desc = "Previous yank" },
			{ "]y", "<Plug>(YankyCycleBackward)", desc = "Next yank" },
		},
	},

	{
		"lmilojevicc/herdr-splits.nvim",
		cond = vim.env.HERDR_ENV == "1",
		opts = { at_edge = "stop", nav_at_edge = "stop", unzoom_on_nav = true },
		keys = {
			{
				"<C-h>",
				function()
					require("herdr-splits").move_cursor_left()
				end,
				desc = "Move to left pane",
			},
			{
				"<C-j>",
				function()
					require("herdr-splits").move_cursor_down()
				end,
				desc = "Move to pane below",
			},
			{
				"<C-k>",
				function()
					require("herdr-splits").move_cursor_up()
				end,
				desc = "Move to pane above",
			},
			{
				"<C-l>",
				function()
					require("herdr-splits").move_cursor_right()
				end,
				desc = "Move to right pane",
			},
		},
	},
}
