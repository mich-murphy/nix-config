local status_ok, treesitter = pcall(require, "treesitter")
if not status_ok then
  return
end

treesitter.setup {
  highlight = {
    enable = true,
    additional_vim_regex_highlighting = false,
  },
  indent = {
    enable = true,
    disable = { "python", "css" }
  },
  autopairs = {
    enable = true,
  },
}
