return {
  {
    "echasnovski/mini.surround",
    opts = {
      mappings = {
        add = "csa", -- Add surrounding in Normal and Visual modes
        delete = "csd", -- Delete surrounding
        find = "csf", -- Find surrounding (to the right)
        find_left = "csF", -- Find surrounding (to the left)
        highlight = "csh", -- Highlight surrounding
        replace = "csr", -- Replace surrounding
        update_n_lines = "csn", -- Update `n_lines`
      }
    }
  },

  {
    "folke/which-key.nvim",
    config = function(_, opts)
        local wk = require("which-key")
        wk.setup(opts)
        local keymaps = {
          mode = { "n", "v" },
          ["g"] = { name = "+goto" },
          ["cs"] = { name = "+surround" },
          ["]"] = { name = "+next" },
          ["["] = { name = "+prev" },
          ["<leader><tab>"] = { name = "+tabs" },
          ["<leader>b"] = { name = "+buffer" },
          ["<leader>c"] = { name = "+code" },
          ["<leader>f"] = { name = "+file/find" },
          ["<leader>g"] = { name = "+git" },
          ["<leader>gh"] = { name = "+hunks" },
          ["<leader>q"] = { name = "+quit/session" },
          ["<leader>s"] = { name = "+search" },
          ["<leader>u"] = { name = "+ui" },
          ["<leader>w"] = { name = "+windows" },
          ["<leader>x"] = { name = "+diagnostics/quickfix" },
          ["<leader>sn"] = { name = "+noice" }
        }
        -- if Util.has("noice.nvim") then
        --   keymaps["<leader>sn"] = { name = "+noice" }
        -- end
        wk.register(keymaps)
      end,
  }
}
