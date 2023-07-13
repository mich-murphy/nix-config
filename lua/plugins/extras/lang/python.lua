return {

  -- add diagnostic and formatter options to null-ls
  {
    "jose-elias-alvarez/null-ls.nvim",
    opts = function(_, opts)
      local nls = require("null-ls")
      if type(opts.sources) == "table" then
        vim.list_extend(opts.sources, {
          nls.builtins.formatting.black,
          nls.builtins.diagnostics.ruff.with({
            extra_args = {
              "--line-length", "88",
            }
          }),
          nls.builtins.formatting.ruff
        })
      end
    end,
  },
}
