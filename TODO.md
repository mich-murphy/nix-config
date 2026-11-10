# TODO

Reviewed: 2026-11-10

This checklist contains only work that remains after the platform-aware Home
Manager refactor.

## Testing

- [ ] **Add targeted checks for important generated configuration**
  - The current flake checks build the Darwin system and the Home Manager
    activation package, but they do not assert the contents of generated
    configuration.
  - Add focused regression checks where a successful build alone would not
    catch unintended changes, prioritising:
    - SSH settings;
    - live configuration links for terminal applications and Karabiner;
    - the declared Homebrew and system package inventory.
  - Keep checks tied to concrete invariants rather than recreating isolated
    module tests for the concern files.
