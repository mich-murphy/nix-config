return {

  -- change notification settings
  {
    "rcarriga/nvim-notify",
    opts = {
      stages = "fade_in_slide_out",
      timeout = 3000,
      render = "compact",
    },
  },

  {
    "nvim-lualine/lualine.nvim",
    event = "VeryLazy",
    opts = function(_, opts)
      opts.options = vim.tbl_extend("force", opts.options, {
        section_separators = "",
        component_separators = "",
      })
    end,
  },

  -- edit pinned neotree options in edgy
  {
    "folke/edgy.nvim",
      opts = function(_, opts)
        opts = {
          bottom = {
            {
              ft = "toggleterm",
              size = { height = 0.4 },
              filter = function(buf, win)
                return vim.api.nvim_win_get_config(win).relative == ""
              end,
            },
            {
              ft = "noice",
              size = { height = 0.4 },
              filter = function(buf, win)
                return vim.api.nvim_win_get_config(win).relative == ""
              end,
            },
            {
              ft = "lazyterm",
              title = "LazyTerm",
              size = { height = 0.4 },
              filter = function(buf)
                return not vim.b[buf].lazyterm_cmd
              end,
            },
            "Trouble",
            { ft = "qf", title = "QuickFix" },
            {
              ft = "help",
              size = { height = 20 },
              -- don't open help files in edgy that we're editing
              filter = function(buf)
                return vim.bo[buf].buftype == "help"
              end,
            },
            { ft = "spectre_panel", size = { height = 0.4 } },
            { title = "Neotest Output", ft = "neotest-output-panel", size = { height = 15 } },
          },
          left = {
            {
              title = "Neo-Tree",
              ft = "neo-tree",
              filter = function(buf)
                return vim.b[buf].neo_tree_source == "filesystem"
              end,
              pinned = true,
              open = function()
                vim.api.nvim_input("<esc><space>e")
              end,
              size = { height = 0.5 },
            },
            { title = "Neotest Summary", ft = "neotest-summary" },
            "neo-tree",
          },
          keys = {
            -- increase width
            ["<c-Right>"] = function(win)
              win:resize("width", 2)
            end,
            -- decrease width
            ["<c-Left>"] = function(win)
              win:resize("width", -2)
            end,
            -- increase height
            ["<c-Up>"] = function(win)
              win:resize("height", 2)
            end,
            -- decrease height
            ["<c-Down>"] = function(win)
              win:resize("height", -2)
            end,
          },
        }
        local Util = require("lazyvim.util")
        if Util.has("symbols-outline.nvim") then
          table.insert(opts.left, {
            title = "Outline",
            ft = "Outline",
            pinned = true,
            open = "SymbolsOutline",
          })
        end
      return opts
    end,
  },

  -- change dashboard logo
  {
    "goolord/alpha-nvim",
    opts = function(_, opts)
      local logo = [[
      ███╗   ██╗ ███████╗  ██████╗  ██╗   ██╗ ██╗ ███╗   ███╗
      ████╗  ██║ ██╔════╝ ██╔═══██╗ ██║   ██║ ██║ ████╗ ████║
      ██╔██╗ ██║ █████╗   ██║   ██║ ██║   ██║ ██║ ██╔████╔██║
      ██║╚██╗██║ ██╔══╝   ██║   ██║ ╚██╗ ██╔╝ ██║ ██║╚██╔╝██║
      ██║ ╚████║ ███████╗ ╚██████╔╝  ╚████╔╝  ██║ ██║ ╚═╝ ██║
      ╚═╝  ╚═══╝ ╚══════╝  ╚═════╝    ╚═══╝   ╚═╝ ╚═╝     ╚═╝

                
               Talk is cheap. Show me the code. 
      ]]
      opts.section.header.val = vim.split(logo, "\n", { trimempty = true })
    end,
  },
}
