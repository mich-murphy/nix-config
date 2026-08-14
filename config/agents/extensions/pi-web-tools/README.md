# Pi Web Tools

A small, hardened Pi extension for public-web research. It is derived from Dillon Mulroy's [`pi-web-tools`](https://github.com/dmmulroy/pi-web-tools) and preserves that project's boundary parsing, typed results, redaction, bounded I/O, output truncation, and test style.

## Public interface

### `websearch`

Searches through Pi's existing OpenAI/Codex authentication, with Exa's standard hosted MCP endpoint (`https://mcp.exa.ai/mcp`) as the bounded fallback. Set `PI_WEB_TOOLS_EXA_ENDPOINT` only to override that default with another public Exa-compatible HTTP(S) endpoint. The extension does not own or inject Exa credentials.

```bash
export PI_WEB_TOOLS_EXA_ENDPOINT="https://your-exa-compatible-gateway.example/mcp"
```

Parameters:

- `query` — required non-empty query
- `maxResults` — optional, clamped to `1..20`
- `depth` — optional provider hint: `auto`, `fast`, or `deep`
- `recency` — optional: `day`, `week`, `month`, or `year`
- `domains` — optional hostname filters; prefix exclusions with `-`

### `webfetch`

Fetches one public HTTP(S) URL as Markdown, text, exact HTML/source, or an inline raster image. Other binary formats, including PDFs, are rejected.

Parameters:

- `url` — required public HTTP(S) URL
- `format` — `markdown` (default), `text`, or `html`
- `timeout` — optional seconds, clamped to `1..120`

`html` mode returns response bodies for non-success HTTP statuses so API and error responses can be inspected. Readable modes reject non-success statuses.

## Security properties

- rejects URL credentials and non-HTTP(S) protocols
- rejects local, private, link-local, documentation, benchmark, multicast, and reserved IP ranges
- pins each connection to the validated DNS result, closing the DNS-rebinding/validation-to-connect gap
- validates every redirect and follows at most five redirects
- limits fetched responses to 5 MB and search responses to 1 MB
- limits model-visible output to Pi's 50 KB / 2,000-line defaults
- writes complete truncated output to a private temporary directory
- does not read browser cookies or local input files
- does not accept or inject search-provider API keys
- does not execute shell credential resolvers, Git, GitHub CLI, browsers, video tools, Python, or SQLite
- does not start a local HTTP server
- uses no hosted page-extraction fallback

Fetched content remains untrusted evidence and must never be treated as agent instructions.

## Development

```bash
npm ci --ignore-scripts
npm run check
npm audit --omit=dev --omit=peer
```
