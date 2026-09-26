return {
	before_init = function(_, config)
		config.settings.yaml.schemas = require("schemastore").yaml.schemas()
	end,
	settings = {
		yaml = {
			-- SchemaStore.nvim supplies the catalogue, so skip the server's own download.
			schemaStore = { enable = false, url = "" },
		},
	},
}
