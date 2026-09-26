-- config/nvim is linked from the nix-config checkout; resolve back to its flake.
local flake_root = vim.fs.dirname(vim.fs.dirname(vim.uv.fs_realpath(vim.fn.stdpath("config"))))
local flake = ('(builtins.getFlake "%s")'):format(flake_root)
local darwin = flake .. ".darwinConfigurations.macbook"

return {
	-- nixd traces every request to stderr at the default level, which fills lsp.log.
	-- Its nixd-attrset-eval workers also outlive it and ignore SIGTERM
	-- (nix-community/nixd#436), so kill the process group once nixd exits.
	cmd = {
		"sh",
		"-c",
		"trap 'pkill -9 -g $$; exit 0' TERM; nixd --log=error <&0 & wait $!; code=$?; pkill -9 -g $$; exit $code",
	},
	on_init = function()
		-- Nvim quits without waiting for servers, and a busy nixd ignores stdin
		-- closing, so signal the wrapper above to take the process group down.
		vim.api.nvim_create_autocmd("VimLeave", {
			group = vim.api.nvim_create_augroup("nixd_process_group", { clear = true }),
			callback = function()
				for _, client in ipairs(vim.lsp.get_clients({ name = "nixd" })) do
					if not client.rpc.is_closing() then
						client.rpc.terminate()
					end
				end
			end,
		})
	end,
	settings = {
		nixd = {
			nixpkgs = { expr = ("import %s.inputs.nixpkgs { }"):format(flake) },
			formatting = { command = { "alejandra" } },
			options = {
				["nix-darwin"] = { expr = darwin .. ".options" },
				["home-manager"] = { expr = darwin .. ".options.home-manager.users.type.getSubOptions [ ]" },
			},
		},
	},
}
