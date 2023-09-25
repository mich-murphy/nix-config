return {

  -- add sql treesitter
  {
    "nvim-treesitter/nvim-treesitter",
    opts = function(_, opts)
      if type(opts.ensure_installed) == "table" then
        vim.list_extend(opts.ensure_installed, { "sql" })
      end
    end,
  },

  -- add lsp extensions to mason
  {
    "williamboman/mason.nvim",
    opts = function(_, opts)
      if type(opts.ensure_installed) == "table" then
        vim.list_extend(opts.ensure_installed, {
          "sqlfluff",
          "sqlfmt"
        })
      end
    end,
  },

  -- add diagnostic and formatter options to null-ls
  {
    "jose-elias-alvarez/null-ls.nvim",
    opts = function(_, opts)
      local nls = require("null-ls")
      table.insert(opts.sources, nls.builtins.diagnostics.sqlfluff)
      table.insert(opts.sources, nls.builtins.formatting.sqlfmt)
    end,
  },
}
