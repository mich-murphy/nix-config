# Global Claude Instructions

## General Workflow

- **Ask before assuming.** Do not make assumptions about design principles, architecture decisions, or user preferences. Ask clarifying questions first, then propose.
- **Front-load debugging context.** When the user provides an error, read the relevant files and analyze the error before suggesting fixes. Do not guess at approaches without evidence.
- **Delegate cross-cutting changes.** When a task involves 3+ independent files or concerns, use sub-agents to handle each independently, then summarize all changes for review before committing.

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

## Azure Infrastructure

- Azure Bastion does **not** support start/stop via REST API — it must be deleted and recreated. Do not attempt `az rest` calls for Bastion start/stop.
- Always verify Azure API endpoints and provider actions exist before implementing. Do not assume RBAC actions or REST paths — check the Azure docs or `az` CLI first.
- Account for RBAC propagation delays (up to several minutes) when troubleshooting permission errors after role assignments.
- PowerShell runtime versions in Azure Automation can drift — verify the runtime is available before referencing it in runbooks.

## Ansible

- `ansible.cfg` does **not** support Jinja2 templating. Use only INI-format values.
- Vault passwords: use `vault_password_file` or `--vault-password-file`, never inline.
- WinRM through Azure Bastion tunnels requires explicit `pywinrm` configuration — verify connectivity before running playbooks.
