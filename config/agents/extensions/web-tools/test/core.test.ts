import { describe, expect, test } from "bun:test";
import {
  MAX_FETCH_BYTES,
  MAX_SEARCH_BYTES,
  selectSearchProvider,
  webFetch,
  webSearch,
} from "../core.js";

describe("webFetch", () => {
  test("converts an HTML response to markdown by default", async () => {
    const requests: Array<{ url: string; init?: RequestInit }> = [];
    const fetchStub = async (input: string | URL | Request, init?: RequestInit) => {
      requests.push({ url: String(input), init });
      return new Response("<h1>Hello</h1><script>bad()</script><p>world</p>", {
        headers: { "content-type": "text/html; charset=utf-8" },
      });
    };

    const result = await webFetch(
      { url: "https://example.test/page" },
      { fetch: fetchStub },
    );

    expect(result).toEqual({
      url: "https://example.test/page",
      contentType: "text/html",
      format: "markdown",
      output: "# Hello\n\nworld",
    });
    expect(requests).toHaveLength(1);
    expect(requests[0]?.init?.headers).toMatchObject({
      Accept: expect.stringContaining("text/markdown"),
      "Accept-Language": "en-US,en;q=0.9",
    });
  });

  test("returns supported images as base64 model content", async () => {
    const result = await webFetch(
      { url: "https://example.test/image.png" },
      {
        fetch: (async () =>
          new Response(new Uint8Array([1, 2, 3]), {
            headers: { "content-type": "image/png" },
          })),
      },
    );

    expect(result.image).toEqual({ data: "AQID", mimeType: "image/png" });
    expect(result.output).toBe("Image fetched successfully");
  });

  test("rejects a declared response larger than 5 MiB", async () => {
    const operation = webFetch(
      { url: "https://example.test/large" },
      {
        fetch: (async () =>
          new Response("small", {
            headers: {
              "content-type": "text/plain",
              "content-length": String(MAX_FETCH_BYTES + 1),
            },
          })),
      },
    );

    await expect(operation).rejects.toThrow("Response exceeds");
  });

  test("retries only a Cloudflare challenge with the fallback user agent", async () => {
    const agents: string[] = [];
    const result = await webFetch(
      { url: "https://example.test/challenge", format: "text" },
      {
        fetch: (async (_input: string | URL | Request, init?: RequestInit) => {
          agents.push((init?.headers as Record<string, string>)["User-Agent"]!);
          if (agents.length === 1) {
            return new Response("challenge", {
              status: 403,
              headers: { "cf-mitigated": "challenge" },
            });
          }
          return new Response("ok", { headers: { "content-type": "text/plain" } });
        }),
      },
    );

    expect(agents).toHaveLength(2);
    expect(agents[1]).toBe("pi");
    expect(result.output).toBe("ok");
  });

  test("rejects unsupported protocols before transport", async () => {
    let called = false;
    const operation = webFetch(
      { url: "file:///etc/passwd" },
      {
        fetch: (async () => {
          called = true;
          return new Response();
        }),
      },
    );

    await expect(operation).rejects.toThrow("Unsupported URL protocol");
    expect(called).toBe(false);
  });
});

describe("webSearch", () => {
  test("calls Exa's MCP search tool and returns its first text result", async () => {
    let request: { url: string; init?: RequestInit } | undefined;
    const fetchStub = async (input: string | URL | Request, init?: RequestInit) => {
      request = { url: String(input), init };
      return Response.json({
        jsonrpc: "2.0",
        id: 1,
        result: { content: [{ type: "text", text: "result text" }] },
      });
    };

    const result = await webSearch(
      { query: "pi extensions", numResults: 3, type: "fast" },
      { provider: "exa", exaApiKey: "secret key" },
      "session-1",
      { fetch: fetchStub },
    );

    expect(result).toEqual({ provider: "exa", text: "result text" });
    expect(request?.url).toBe("https://mcp.exa.ai/mcp?exaApiKey=secret+key");
    expect(JSON.parse(String(request?.init?.body))).toEqual({
      jsonrpc: "2.0",
      id: 1,
      method: "tools/call",
      params: {
        name: "web_search_exa",
        arguments: {
          query: "pi extensions",
          type: "fast",
          numResults: 3,
          livecrawl: "fallback",
        },
      },
    });
  });

  test("calls Parallel with authorization and parses an SSE response", async () => {
    let request: { url: string; init?: RequestInit } | undefined;
    const result = await webSearch(
      { query: "current news" },
      { provider: "parallel", parallelApiKey: "parallel-secret" },
      "session-2",
      {
        fetch: (async (input: string | URL | Request, init?: RequestInit) => {
          request = { url: String(input), init };
          return new Response(
            'event: message\ndata: {"result":{"content":[{"type":"text","text":"SSE result"}]}}\n\n',
            { headers: { "content-type": "text/event-stream" } },
          );
        }),
      },
    );

    expect(result).toEqual({ provider: "parallel", text: "SSE result" });
    expect(request?.url).toBe("https://search.parallel.ai/mcp");
    expect(request?.init?.headers).toMatchObject({
      Authorization: "Bearer parallel-secret",
      Accept: "application/json, text/event-stream",
    });
    expect(JSON.parse(String(request?.init?.body)).params.arguments).toEqual({
      objective: "current news",
      search_queries: ["current news"],
      session_id: "session-2",
    });
  });

  test("validates numeric bounds before contacting a provider", async () => {
    let called = false;
    const operation = webSearch(
      { query: "query", numResults: 21 },
      { provider: "exa" },
      "session",
      {
        fetch: (async () => {
          called = true;
          return Response.json({});
        }),
      },
    );

    await expect(operation).rejects.toThrow("numResults");
    expect(called).toBe(false);
  });

  test("rejects search responses larger than 256 KiB", async () => {
    const operation = webSearch(
      { query: "query" },
      { provider: "exa" },
      "session",
      {
        fetch: (async () =>
          new Response("x", {
            headers: { "content-length": String(MAX_SEARCH_BYTES + 1) },
          })),
      },
    );

    await expect(operation).rejects.toThrow("Response exceeds");
  });

  test("uses explicit and enabled-provider precedence", () => {
    expect(selectSearchProvider({ provider: "exa", enableParallel: true }, "s")).toBe("exa");
    expect(selectSearchProvider({ enableExa: true, enableParallel: true }, "s")).toBe("parallel");
    expect(selectSearchProvider({ enableExa: true }, "s")).toBe("exa");
    expect(selectSearchProvider({}, "stable-session")).toBe(
      selectSearchProvider({}, "stable-session"),
    );
  });
});
