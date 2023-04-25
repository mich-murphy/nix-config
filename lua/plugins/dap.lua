return {
  'mfussenegger/nvim-dap',

  dependencies = {
    'rcarriga/nvim-dap-ui',
    'williamboman/mason.nvim',
    'jay-babu/mason-nvim-dap.nvim',
    'theHamsta/nvim-dap-virtual-text',
    'mfussenegger/nvim-dap-python',
  },

  config = function()
    local dap = require 'dap'
    local dapui = require 'dapui'
    local dappy = require 'dap-python'

    require('mason-nvim-dap').setup {
      -- Makes a best effort to setup the various debuggers with
      -- reasonable debug configurations
      automatic_setup = true,
      ensure_installed = {
        -- Update this to ensure that you have the debuggers for the langs you want
        'debugpy',
      },
    }

    -- You can provide additional configuration to the handlers,
    -- see mason-nvim-dap README for more information
    -- require('mason-nvim-dap').setup_handlers()

    -- Basic debugging keymaps
    vim.keymap.set('n', '<leader>d<space>', dap.continue, { desc = "Continue"})
    vim.keymap.set('n', '<leader>dl', dap.step_into, { desc = "Step into"})
    vim.keymap.set('n', '<leader>dk', dap.step_over, { desc = "Step over"})
    vim.keymap.set('n', '<leader>dh', dap.step_out, { desc = "Step out"})
    vim.keymap.set('n', '<leader>dm', dappy.test_method, { desc = "Test Python method"})
    vim.keymap.set('n', '<leader>dc', dappy.test_class, { desc = "Test Python class"})
    vim.keymap.set('n', '<leader>ds', dappy.debug_selection, { desc = "Debug selection"})
    require("which-key").register({
      ["<leader>d"] = {
        name = "+debug",
      }
    })
    vim.keymap.set('n', '<leader>db', dap.toggle_breakpoint, { desc = "Toggle breakpoint"})
    vim.keymap.set('n', '<leader>dB', function()
      dap.set_breakpoint(vim.fn.input 'Breakpoint condition: ')
    end, { desc = "Toggle breakpoint condition" })

    -- Dap UI setup
    -- For more information, see |:help nvim-dap-ui|
    dapui.setup {
      icons = { expanded = '▾', collapsed = '▸', current_frame = '*' },
      controls = {
        icons = {
          pause = '⏸',
          play = '▶',
          step_into = '⏎',
          step_over = '⏭',
          step_out = '⏮',
          step_back = 'b',
          run_last = '▶▶',
          terminate = '⏹',
        },
      },
    }

    dap.listeners.after.event_initialized['dapui_config'] = dapui.open
    dap.listeners.before.event_terminated['dapui_config'] = dapui.close
    dap.listeners.before.event_exited['dapui_config'] = dapui.close

    -- Install python specific config
    require('dap-python').setup()
  end,
}
