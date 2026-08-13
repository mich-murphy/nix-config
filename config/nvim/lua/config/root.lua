local M = {}

local function normalize(path)
	return type(path) == "string" and path ~= "" and vim.fs.normalize(path) or nil
end

local function buffer_path(buf)
	local name = vim.api.nvim_buf_get_name(buf or 0)
	if name == "" or vim.bo[buf or 0].buftype ~= "" then
		return nil
	end
	return normalize(name)
end

local function contains(root, path)
	return path == root or path:sub(1, #root + 1) == root .. "/"
end

local function lsp_root(buf, path)
	local roots = {}

	for _, client in ipairs(vim.lsp.get_clients({ bufnr = buf })) do
		for _, folder in ipairs(client.workspace_folders or {}) do
			roots[#roots + 1] = normalize(vim.uri_to_fname(folder.uri))
		end
		roots[#roots + 1] = normalize(client.config.root_dir)
	end

	local best
	for _, root in ipairs(roots) do
		if root and path and contains(root, path) and (not best or #root > #best) then
			best = root
		end
	end
	return best
end

function M.git(buf)
	local path = buffer_path(buf) or normalize(vim.uv.cwd())
	local start = path and (vim.fn.isdirectory(path) == 1 and path or vim.fs.dirname(path))
	return (start and vim.fs.root(start, ".git")) or M.get(buf)
end

function M.get(buf)
	buf = buf or 0
	local path = buffer_path(buf)
	local root = lsp_root(buf, path)
	if root then
		return root
	end

	local start = path and vim.fs.dirname(path)
	return normalize((start and vim.fs.root(start, ".git")) or vim.uv.cwd() or ".")
end

return M
