// Serves the markdown twin of a docs page when a client explicitly asks for
// text/markdown, so agents can read the docs without scraping HTML.
//
// The mapping mirrors the static export: every docs page has an asset at
// /llms.mdx/docs/<slug>/content.md, and the landing page at
// /llms.mdx/docs/content.md. Only requests that name text/markdown in Accept
// negotiate; browsers never do, so they keep the HTML untouched.
interface Env {
  ASSETS: {
    fetch(request: RequestInfo | URL): Promise<Response>;
  };
}

const MARKDOWN_TYPE = "text/markdown; charset=utf-8";

function markdownAssetPath(pathname: string): string | null {
  const path =
    pathname !== "/" && pathname.endsWith("/")
      ? pathname.slice(0, -1)
      : pathname;
  if (path === "/docs") return "/llms.mdx/docs/content.md";
  if (path.startsWith("/docs/")) return `/llms.mdx${path}/content.md`;
  return null;
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    const assetPath = markdownAssetPath(url.pathname);

    if (
      assetPath &&
      (request.headers.get("Accept") ?? "").includes("text/markdown")
    ) {
      const markdown = await env.ASSETS.fetch(new URL(assetPath, url.origin));
      if (markdown.status === 404) return env.ASSETS.fetch(request);

      const response = new Response(markdown.body, markdown);
      response.headers.set("Content-Type", MARKDOWN_TYPE);
      response.headers.set("Vary", "Accept");
      return response;
    }

    const asset = await env.ASSETS.fetch(request);
    if (!assetPath) return asset;

    const response = new Response(asset.body, asset);
    response.headers.append("Vary", "Accept");
    return response;
  },
};
