# Global agent instructions

- Wait on CI with `gh run watch <run-id> --interval 60 --exit-status`, not `sleep` or polling loops. Run one watcher at a time per session and give subagents the same limit. On a rate-limit 403, wait for the reset instead of retrying. Use `gh run list --branch <branch>`, not `gh pr checks`.
- Group related changes into a stack with `gh stack` by default.
- When a step needs a human action (an approval, a plan or a deployment gate), post its full URL as soon as it is needed.
