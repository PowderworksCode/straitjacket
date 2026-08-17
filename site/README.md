# straitjacket docs site

The documentation site for [Straitjacket](../README.md), served at
[straitjacket.dev](https://straitjacket.dev). Built with
[Fumadocs](https://fumadocs.dev) (Next.js, static export) and structured around
[Diátaxis](https://diataxis.fr/).

## Develop

```sh
bun install
bun run dev
```

Open http://localhost:3000.

Content lives in `content/docs/` as MDX, organized into the four Diátaxis
quadrants — `tutorials/`, `guides/`, `reference/`, `explanation/`. Sidebar order
is controlled by the `meta.json` in each folder.

## Build

```sh
bun run build   # static export to ./out
```

The whole site is prerendered to static HTML in `out/` (`output: 'export'` in
`next.config.mjs`). Search is a static Orama index — no server needed.

## Deploy (Cloudflare Workers)

The site is a static export, so it deploys as an assets-only Worker with no
runtime code. `wrangler.toml` points `[assets]` at `./out`.

**Via the dashboard** — connect the repo and set:

| setting | value |
| --- | --- |
| Root directory | `site` |
| Build command | `bun run build` |
| Deploy command | `npx wrangler deploy` |

**Locally:**

```sh
bun run build
bunx wrangler deploy
```

Point the `straitjacket.dev` custom domain at the Worker in the Cloudflare
dashboard.
