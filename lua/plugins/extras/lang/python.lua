return {

  -- add python treesitter
  {
    "nvim-treesitter/nvim-treesitter",
    opts = function(_, opts)
      vim.list_extend(opts.ensure_installed, { "python" })
    end,
  },

  -- add lsp and dap extensions to mason
  {
    "williamboman/mason.nvim",
    opts = function(_, opts)
      vim.list_extend(opts.ensure_installed, { "pyright", "flake8", "black", "debugpy" })
    end,
  },

  -- add diagnostic and formatter options to null-ls
  {
    "jose-elias-alvarez/null-ls.nvim",
    opts = function()
      local nls = require("null-ls")
      return {
        sources = {
          -- nls.builtins.formatting.prettierd,
          nls.builtins.formatting.stylua,
          nls.builtins.formatting.black.with({
            prefer_local = "./.virtualenv/bin/black",
          }),
          nls.builtins.diagnostics.flake8.with({
            prefer_local = "./.virtualenv/bin/flake8",
            extra_args = {
              "--max-line-length", "120",
            }
          }),
        },
      }
    end,
  },

  -- add lsp server for python
  {
    "neovim/nvim-lspconfig",
    opts = {
      servers = {
        pyright = {},
      },
    },
  },

  -- dap configuration for python
  {
    "mfussenegger/nvim-dap",
    dependencies = {
    {
      "mfussenegger/nvim-dap-python",
      keys = {
          { "<leader>dm", function() require("dap-python").test_method() end, desc = "Test Python method"},
          { "<leader>dc", function() require("dap-python").test_class() end, desc = "Test Python class"},
      },
      config = function()
        local dappy = require("dap-python")
        dappy.setup("./.virtualenv/bin/python")
        dappy.test_runner = "pytest"
      end,
      },
    }
  }
}
