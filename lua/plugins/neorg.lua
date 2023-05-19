return {
  {
    "nvim-neorg/neorg",
    ft = "norg",
    dependencies = { "nvim-lua/plenary.nvim" },
    build = ":Neorg sync-parsers",
    opts = {
      load = {
        ["core.defaults"] = {}, -- Loads default behaviour
        ["core.concealer"] = {}, -- Adds pretty icons to your documents
        ["core.pivot"] = {}, -- Adds pretty icons to your documents
        ["core.itero"] = {}, -- Adds pretty icons to your documents
        ["core.promo"] = {}, -- Adds pretty icons to your documents
        ["core.dirman"] = { -- Manages Neorg workspaces
          config = {
            workspaces = {
              notes = "~/Documents/notes",
            },
          },
        },
      },
    },
  }
}
