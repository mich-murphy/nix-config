local function augroup(name)
	return vim.api.nvim_create_augroup("nvim_" .. name, { clear = true })
end

vim.diagnostic.config({
	underline = true,
	update_in_insert = false,
	virtual_lines = true,
	virtual_text = false,
	severity_sort = true,
	signs = {
		text = {
			[vim.diagnostic.severity.ERROR] = "",
			[vim.diagnostic.severity.WARN] = "",
			[vim.diagnostic.severity.HINT] = "",
			[vim.diagnostic.severity.INFO] = "",
		},
	},
})

vim.api.nvim_create_autocmd({ "FocusGained", "TermClose", "TermLeave" }, {
	group = augroup("checktime"),
	callback = function()
		if vim.bo.buftype ~= "nofile" then
			vim.cmd.checktime()
		end
	end,
})

vim.api.nvim_create_autocmd("TextYankPost", {
	group = augroup("highlight_yank"),
	callback = function()
		(vim.hl or vim.highlight).on_yank()
	end,
})

vim.api.nvim_create_autocmd("VimResized", {
	group = augroup("resize_splits"),
	callback = function()
		local tab = vim.fn.tabpagenr()
		vim.cmd.tabdo("wincmd =")
		vim.cmd.tabnext(tab)
	end,
})

vim.api.nvim_create_autocmd("BufReadPost", {
	group = augroup("last_location"),
	callback = function(event)
		if vim.bo[event.buf].filetype == "gitcommit" then
			return
		end
		local mark = vim.api.nvim_buf_get_mark(event.buf, '"')
		if mark[1] > 0 and mark[1] <= vim.api.nvim_buf_line_count(event.buf) then
			pcall(vim.api.nvim_win_set_cursor, 0, mark)
		end
	end,
})

vim.api.nvim_create_autocmd("FileType", {
	group = augroup("close_with_q"),
	pattern = {
		"checkhealth",
		"gitsigns-blame",
		"grug-far",
		"help",
		"lazy",
		"lspinfo",
		"mason",
		"notify",
		"qf",
		"snacks_notif",
	},
	callback = function(event)
		vim.bo[event.buf].buflisted = false
		vim.schedule(function()
			vim.keymap.set("n", "q", function()
				pcall(vim.cmd.close)
				pcall(vim.api.nvim_buf_delete, event.buf, { force = true })
			end, { buffer = event.buf, desc = "Close buffer", silent = true })
		end)
	end,
})

vim.api.nvim_create_autocmd("FileType", {
	group = augroup("man_unlisted"),
	pattern = "man",
	callback = function(event)
		vim.bo[event.buf].buflisted = false
	end,
})

vim.api.nvim_create_autocmd("FileType", {
	group = augroup("wrap_spell"),
	pattern = { "text", "plaintex", "typst", "gitcommit", "markdown" },
	callback = function()
		vim.opt_local.wrap = true
		vim.opt_local.spell = true
	end,
})

vim.api.nvim_create_autocmd("FileType", {
	group = augroup("json_conceal"),
	pattern = { "json", "jsonc", "json5" },
	callback = function()
		vim.opt_local.conceallevel = 0
	end,
})

vim.api.nvim_create_autocmd("BufWritePre", {
	group = augroup("create_parent_directory"),
	callback = function(event)
		if event.match:match("^%w[%w+.-]*://") then
			return
		end
		local file = vim.uv.fs_realpath(event.match) or event.match
		vim.fn.mkdir(vim.fn.fnamemodify(file, ":p:h"), "p")
	end,
})

vim.filetype.add({
	extension = {
		gotmpl = "gotmpl",
		mdx = "markdown.mdx",
	},
	filename = {
		[".gitlab-ci.yml"] = "yaml.gitlab",
		[".gitlab-ci.yaml"] = "yaml.gitlab",
		["docker-compose.yml"] = "yaml.docker-compose",
		["docker-compose.yaml"] = "yaml.docker-compose",
	},
	pattern = {
		[".*/compose%.ya?ml"] = "yaml.docker-compose",
		[".*/values[^/]*%.ya?ml"] = "yaml.helm-values",
	},
})

vim.api.nvim_create_autocmd("LspAttach", {
	group = augroup("lsp_attach"),
	callback = function(event)
		local client = assert(vim.lsp.get_client_by_id(event.data.client_id))
		local buf = event.buf
		local function map(lhs, rhs, desc, mode)
			vim.keymap.set(mode or "n", lhs, rhs, { buffer = buf, desc = desc, silent = true })
		end
		local function supports(method)
			return client:supports_method(method, buf)
		end

		if supports("textDocument/definition") then
			map("gd", function()
				Snacks.picker.lsp_definitions()
			end, "Go to definition")
		end
		if supports("textDocument/references") then
			map("grr", function()
				Snacks.picker.lsp_references()
			end, "References")
		end
		if supports("textDocument/implementation") then
			map("gI", function()
				Snacks.picker.lsp_implementations()
			end, "Go to implementation")
		end
		if supports("textDocument/typeDefinition") then
			map("gy", function()
				Snacks.picker.lsp_type_definitions()
			end, "Go to type definition")
		end
		if supports("textDocument/declaration") then
			map("gD", vim.lsp.buf.declaration, "Go to declaration")
		end
		if supports("textDocument/hover") then
			map("K", vim.lsp.buf.hover, "Hover")
		end
		if supports("textDocument/signatureHelp") then
			map("gK", vim.lsp.buf.signature_help, "Signature help")
			map("<C-k>", vim.lsp.buf.signature_help, "Signature help", "i")
		end
		if supports("textDocument/codeAction") then
			map("<leader>ca", vim.lsp.buf.code_action, "Code action", { "n", "x" })
			map("<leader>co", function()
				vim.lsp.buf.code_action({
					apply = true,
					context = { only = { "source.organizeImports" }, diagnostics = {} },
				})
			end, "Organize imports")
		end
		if supports("textDocument/rename") then
			map("<leader>cr", vim.lsp.buf.rename, "Rename symbol")
		end
		if supports("workspace/willRenameFiles") or supports("workspace/didRenameFiles") then
			map("<leader>cR", function()
				Snacks.rename.rename_file()
			end, "Rename file")
		end
		if supports("textDocument/documentSymbol") then
			map("<leader>ss", function()
				Snacks.picker.lsp_symbols()
			end, "Document symbols")
		end
		if supports("workspace/symbol") then
			map("<leader>sS", function()
				Snacks.picker.lsp_workspace_symbols()
			end, "Workspace symbols")
		end

		map("<leader>cl", function()
			Snacks.picker.lsp_config()
		end, "LSP configuration")

		if supports("textDocument/inlayHint") then
			vim.lsp.inlay_hint.enable(true, { bufnr = buf })
		end
		if supports("textDocument/foldingRange") then
			for _, win in ipairs(vim.fn.win_findbuf(buf)) do
				vim.wo[win].foldmethod = "expr"
				vim.wo[win].foldexpr = "v:lua.vim.lsp.foldexpr()"
			end
		end
	end,
})
