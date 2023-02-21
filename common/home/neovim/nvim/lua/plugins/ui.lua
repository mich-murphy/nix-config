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
      ]]
      opts.section.header.val = vim.split(logo, "\n", { trimempty = true })
    end,
  },
}
