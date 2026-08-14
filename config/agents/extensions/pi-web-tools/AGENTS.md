# Pi Web Tools

## Guardrails

- Parse external data at boundaries before passing it inward.
- Preserve SSRF protections, URL credential redaction, response limits, and output truncation.
- Model expected failures with the local `Result` type; throw at the Pi adapter only to mark a tool execution as failed.
- Put runtime libraries in `dependencies` and Pi core packages in `peerDependencies`.
- Keep the public interface to `websearch` and `webfetch`; put provider and extraction complexity behind those deep modules.
- Do not add browser-cookie access, local-file input, shell command execution, embedded servers, or hosted page-extraction fallbacks.
- Search providers are internal adapters. Do not expose provider selection or credentials through the tool interface.
- Run `npm run check` and `npm audit --omit=dev --omit=peer` after changes.
