local root = require("config.root")

local function picker(name, get_opts)
	return function()
		if get_opts then
			Snacks.picker[name](get_opts())
		else
			Snacks.picker[name]()
		end
	end
end

local function project_picker(name)
	return picker(name, function()
		return { cwd = root.get() }
	end)
end

local function toggle_explorer(open)
	return function()
		local explorer = Snacks.picker.get({ source = "explorer" })[1]
		if explorer then
			explorer:close()
		else
			open()
		end
	end
end

return {
	{
		"folke/snacks.nvim",
		priority = 1000,
		lazy = false,
		init = function()
			if vim.env.TERM_PROGRAM == "ghostty" then
				vim.env.SNACKS_GHOSTTY = "true"
			end
		end,
		opts = {
			bigfile = { enabled = true },
			explorer = {
				enabled = true,
				replace_netrw = true,
				trash = true,
			},
			indent = { enabled = true },
			image = { enabled = true },
			input = { enabled = true },
			notifier = { enabled = true, style = "compact", timeout = 3000 },
			picker = { enabled = true },
			quickfile = { enabled = true },
			scope = { enabled = true },
			statuscolumn = { enabled = true },
			words = { enabled = true },
		},
		config = function(_, opts)
			require("snacks").setup(opts)

			Snacks.toggle({
				name = "Autoformat",
				get = function()
					return vim.g.autoformat ~= false
				end,
				set = function(state)
					vim.g.autoformat = state
				end,
			}):map("<leader>uf")
			Snacks.toggle({
				name = "Buffer autoformat",
				get = function()
					return vim.b.autoformat == nil and vim.g.autoformat ~= false or vim.b.autoformat
				end,
				set = function(state)
					vim.b.autoformat = state
				end,
			}):map("<leader>uF")
			Snacks.toggle.option("spell", { name = "Spelling" }):map("<leader>us")
			Snacks.toggle.option("wrap", { name = "Wrap" }):map("<leader>uw")
			Snacks.toggle.option("relativenumber", { name = "Relative numbers" }):map("<leader>un")
			Snacks.toggle.diagnostics():map("<leader>ud")
			Snacks.toggle
				.option("conceallevel", {
					name = "Conceal",
					off = 0,
					on = vim.o.conceallevel > 0 and vim.o.conceallevel or 2,
				})
				:map("<leader>uc")
			Snacks.toggle.treesitter():map("<leader>uT")
			Snacks.toggle.inlay_hints():map("<leader>uh")
			Snacks.toggle.zoom():map("<leader>wm")
		end,
		keys = {
			{
				"<leader>e",
				toggle_explorer(function()
					Snacks.explorer.reveal()
				end),
				desc = "Explore current file",
			},
			{
				"<leader>E",
				toggle_explorer(function()
					Snacks.explorer({ cwd = root.get() })
				end),
				desc = "Explore project",
			},
			{
				"[[",
				function()
					Snacks.words.jump(-vim.v.count1)
				end,
				desc = "Previous reference",
			},
			{
				"]]",
				function()
					Snacks.words.jump(vim.v.count1)
				end,
				desc = "Next reference",
			},
			{ "<leader><space>", project_picker("files"), desc = "Find project files" },
			{ "<leader>/", project_picker("grep"), desc = "Grep project" },
			{ "<leader>:", picker("command_history"), desc = "Command history" },
			{ "<leader>n", picker("notifications"), desc = "Notification history" },

			{ "<leader>fb", picker("buffers"), desc = "Buffers" },
			{
				"<leader>fB",
				picker("buffers", function()
					return { hidden = true, nofile = true }
				end),
				desc = "All buffers",
			},
			{
				"<leader>fc",
				picker("files", function()
					return { cwd = vim.fn.stdpath("config") }
				end),
				desc = "Configuration files",
			},
			{ "<leader>ff", project_picker("files"), desc = "Find project files" },
			{
				"<leader>fF",
				picker("files", function()
					return { cwd = vim.uv.cwd() }
				end),
				desc = "Find working-directory files",
			},
			{
				"<leader>fg",
				picker("git_files", function()
					return { cwd = root.git() }
				end),
				desc = "Tracked files",
			},
			{ "<leader>fp", picker("projects"), desc = "Projects" },
			{ "<leader>fr", picker("recent"), desc = "Recent files" },
			{
				"<leader>fR",
				picker("recent", function()
					return { filter = { cwd = root.get() } }
				end),
				desc = "Recent project files",
			},

			{ "<leader>sb", picker("lines"), desc = "Buffer lines" },
			{ "<leader>sB", picker("grep_buffers"), desc = "Grep open buffers" },
			{ "<leader>sg", project_picker("grep"), desc = "Grep project" },
			{
				"<leader>sG",
				picker("grep", function()
					return { cwd = vim.uv.cwd() }
				end),
				desc = "Grep working directory",
			},
			{ "<leader>sw", project_picker("grep_word"), mode = { "n", "x" }, desc = "Grep word or selection" },
			{
				"<leader>sW",
				picker("grep_word", function()
					return { cwd = vim.uv.cwd() }
				end),
				mode = { "n", "x" },
				desc = "Grep word in working directory",
			},
			{ '<leader>s"', picker("registers"), desc = "Registers" },
			{ "<leader>s/", picker("search_history"), desc = "Search history" },
			{ "<leader>sa", picker("autocmds"), desc = "Autocommands" },
			{ "<leader>sc", picker("command_history"), desc = "Command history" },
			{ "<leader>sC", picker("commands"), desc = "Commands" },
			{ "<leader>sd", picker("diagnostics"), desc = "Workspace diagnostics" },
			{ "<leader>sD", picker("diagnostics_buffer"), desc = "Buffer diagnostics" },
			{ "<leader>sh", picker("help"), desc = "Help" },
			{ "<leader>sj", picker("jumps"), desc = "Jumps" },
			{ "<leader>sk", picker("keymaps"), desc = "Keymaps" },
			{ "<leader>sM", picker("man"), desc = "Manual pages" },
			{ "<leader>sm", picker("marks"), desc = "Marks" },
			{ "<leader>sR", picker("resume"), desc = "Resume picker" },
			{ "<leader>su", picker("undo"), desc = "Undo history" },

			{
				"<leader>bd",
				function()
					Snacks.bufdelete()
				end,
				desc = "Delete buffer",
			},
			{
				"<leader>bo",
				function()
					Snacks.bufdelete.other()
				end,
				desc = "Delete other buffers",
			},
			{
				"<leader>bi",
				function()
					Snacks.bufdelete.invisible()
				end,
				desc = "Delete invisible buffers",
			},
			{ "<leader>bD", "<cmd>bdelete<cr>", desc = "Delete buffer and window" },

			{
				"<leader>gg",
				function()
					Snacks.lazygit({ cwd = root.git() })
				end,
				desc = "LazyGit",
			},
			{
				"<leader>gl",
				picker("git_log", function()
					return { cwd = root.git() }
				end),
				desc = "Git log",
			},
			{ "<leader>gb", picker("git_log_line"), desc = "Current-line history" },
			{ "<leader>gf", picker("git_log_file"), desc = "Current-file history" },
			{
				"<leader>gB",
				function()
					Snacks.gitbrowse()
				end,
				mode = { "n", "x" },
				desc = "Open Git remote",
			},
			{
				"<leader>gY",
				function()
					Snacks.gitbrowse({
						notify = false,
						open = function(url)
							vim.fn.setreg("+", url)
							vim.notify("Copied Git remote URL")
						end,
					})
				end,
				mode = { "n", "x" },
				desc = "Copy Git remote URL",
			},

			{
				"<localleader>r",
				function()
					Snacks.debug.run()
				end,
				ft = "lua",
				mode = { "n", "x" },
				desc = "Run Lua",
			},
		},
	},
}
