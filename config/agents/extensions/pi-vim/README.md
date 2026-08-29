# Pi Vim editor

Adds a small Vim-style normal mode to Pi's main prompt editor without replacing
Pi's editor implementation.

The editor starts in Insert mode and returns to Insert mode after a successful
submission. Press `Esc` for Normal mode. The editor border shows the current
mode.

## Normal-mode bindings

- Movement: `h`, `j`, `k`, `l`, `w`, `b`, `0`, `$`
- Insert: `i`, `a`, `A`, `I`, `o`, `O`
- Editing: `x`, `D`, `C`, `u`, `dd`, `dw`, `diw`, `cc`, `cw`, `ciw`

Normal-mode `Esc` retains Pi's interrupt behavior. Other Pi application
shortcuts continue to use the configured Pi keybindings.

Word operations intentionally use Pi's existing word boundaries. `dd` is made
from Pi's delete-to-line-end and forward-delete actions, so restoring it may
require two undo operations.
