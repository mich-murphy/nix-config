-- nixd settings for this flake. Neovim loads this through 'exrc' once it is
-- trusted (:trust); the generic nixd setup lives in ~/.config/nvim.
local root = vim.fs.dirname(debug.getinfo(1, "S").source:sub(2))
local flake = ('(builtins.getFlake "%s")'):format(root)
-- Each Mac's host name is its flake name.
local host = vim.uv.os_gethostname():match("^[^.]+")
local darwin = ("%s.darwinConfigurations.%s"):format(flake, host)

vim.lsp.config("nixd", {
	settings = {
		nixd = {
			nixpkgs = { expr = ("import %s.inputs.nixpkgs { }"):format(flake) },
			options = {
				["nix-darwin"] = { expr = darwin .. ".options" },
				["home-manager"] = { expr = darwin .. ".options.home-manager.users.type.getSubOptions [ ]" },
			},
		},
	},
})
