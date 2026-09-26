return {
	before_init = function(_, config)
		config.settings.json.schemas = require("schemastore").json.schemas()
	end,
	settings = {
		json = { validate = { enable = true } },
	},
}
