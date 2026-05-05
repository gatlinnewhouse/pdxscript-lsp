-- PDX script filetype settings for Neovim
-- Place at: ~/.config/nvim/after/ftplugin/pdxscript.lua

-- Line comments use #
vim.bo.commentstring = "#%s"

-- Optional: tab/indent settings common in PDX script files
vim.bo.tabstop = 4
vim.bo.shiftwidth = 4
vim.bo.expandtab = false
