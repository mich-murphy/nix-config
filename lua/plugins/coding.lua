return {
  {

    -- remap surround keys
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
  }
}
