# Global agent instructions

- Wait on CI with `gh run watch <run-id> --exit-status` or `gh pr checks <pr> --watch --fail-fast`, not `sleep` or polling loops.
- Group related changes into a stack with `gh stack` by default.
- When a step needs a human action (an approval, a plan or a deployment gate), post its full URL as soon as it is needed.
