# Pi Web Tools

A global Pi Coding Agent extension that registers two local tools:

- `webfetch` — direct HTTP/HTTPS fetch with HTML-to-Markdown/text conversion and image support.
- `websearch` — Exa or Parallel web search through their hosted MCP `tools/call` endpoints.

Run `/reload` in an existing Pi session to load the extension.

## Configuration

Search works with the providers' unauthenticated behavior where available. Optional credentials:

```sh
export EXA_API_KEY=...
export PARALLEL_API_KEY=...
```

Force a provider with `PI_WEBSEARCH_PROVIDER=exa|parallel`. For compatibility with the studied OpenCode behavior, `OPENCODE_WEBSEARCH_PROVIDER` and its Exa/Parallel enable flags are also recognized.

Provider selection order is: explicit provider, Parallel enable flag, Exa enable flag, then a deterministic split based on the Pi session ID.

## Limits and behavior

- Fetch timeout defaults to 30 seconds and accepts values up to 120 seconds.
- Fetch bodies are stream-limited to 5 MiB.
- Search responses are stream-limited to 256 KiB with a 25-second timeout.
- Model-visible text is limited to Pi's standard 2,000 lines or 50 KiB. Full truncated text is written to a mode-`0600` temporary file and its path is returned.
- HTML can be returned unchanged or converted to Markdown/plain text.
- Supported image responses are returned as Pi image content.
- Search accepts direct JSON and line-oriented SSE MCP responses.
- Abort signals from Pi are wired into both network operations.

`webfetch` is not a browser: it does not execute JavaScript, maintain a cookie jar, or render a DOM. It deliberately permits localhost, private-network URLs, and redirects, matching the developer-agent behavior described in the source study. Do not expose this extension in an untrusted remote or multi-tenant environment without an SSRF policy.

## Development

```sh
npm test
npm run check
```
