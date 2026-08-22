// Serves the markdown twin of a docs page when a client explicitly asks for
// text/markdown. powderworks-docs writes each page's source as index.md
// beside its rendered index.html, so negotiation is a path rewrite.
//
// Only requests that name text/markdown in Accept negotiate; browsers never
// do, so they keep the HTML untouched.

interface Env {
  ASSETS: {
    fetch(request: RequestInfo | URL): Promise<Response>;
  };
}

const MARKDOWN_TYPE = "text/markdown; charset=utf-8";

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    const asset = await env.ASSETS.fetch(request);
    if (!(request.headers.get("Accept") ?? "").includes("text/markdown"))
      return asset;

    const path = url.pathname !== "/" && url.pathname.endsWith("/")
      ? url.pathname.slice(0, -1)
      : url.pathname;
    const twin = await env.ASSETS.fetch(new URL(`${path}/index.md`, url.origin));
    if (twin.status === 404) return asset;

    const response = new Response(twin.body, twin);
    response.headers.set("Content-Type", MARKDOWN_TYPE);
    response.headers.set("Vary", "Accept");
    return response;
  },
};
