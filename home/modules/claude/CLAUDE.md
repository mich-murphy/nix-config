# Global Claude Instructions

## User context
- macOS aarch64-darwin, Nix-managed system (Determinate Systems installer)
- Shell: fish
- Editor: neovim
- All personal machines use nix-darwin + home-manager

## Preferences
- Be concise. No trailing summaries.
- Conventional Commits for all repos: fix(scope): msg, feat(scope): msg
- Always verify builds before suggesting switch/deploy.
- Prefer Nix packages over Homebrew when available.
- Format Nix files with alejandra (nix fmt).
- Do not add comments, docstrings, or type annotations to code you didn't change.
