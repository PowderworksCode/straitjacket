// The browser tab is the one piece of a page no test could see before: the
// frontmatter suite reads sources, and every rendered <title> said "undefined"
// while the suite stayed green. So this builds the site and reads the output.
//
// It runs the build command out of package.json and reads the tab the registry
// names, rather than restating either. Twice this suite restated the flags and
// twice they drifted, staying green against a site unlike the published one.
// A tab and its heading come from one field by two paths, so asserting they
// agree catches either path dropping it; where a tab-title is given, in
// frontmatter or in powderworks.toml, the tab must be exactly that.
import { afterAll, describe, expect, test } from "bun:test";
import { mkdtempSync, readdirSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const SITE = join(import.meta.dir, "..");
const SITE_NAME = "Straitjacket";
const out = mkdtempSync(join(tmpdir(), "straitjacket-titles-"));
afterAll(() => rmSync(out, { recursive: true, force: true }));

const script = JSON.parse(readFileSync(join(SITE, "package.json"), "utf8"))
  .scripts.build;
const run = Bun.spawnSync(
  [
    "sh",
    "-c",
    script.replace("rm -rf out && ", "").replace("--out out", `--out '${out}'`),
  ],
  {
    cwd: SITE,
    env: {
      ...process.env,
      PATH: `${join(SITE, "node_modules", ".bin")}:${process.env.PATH}`,
    },
  },
);

function tabTitlesByEmittedUrl(
  dir: string,
  trail: string[] = [],
): Map<string, string> {
  const found = new Map<string, string>();
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.isDirectory()) {
      for (const [url, value] of tabTitlesByEmittedUrl(join(dir, entry.name), [
        ...trail,
        entry.name,
      ]))
        found.set(url, value);
    } else if (entry.name.endsWith(".md")) {
      const slug = entry.name.slice(0, -3);
      const declared = /^tab-title:\s*(.+)$/m.exec(
        readFileSync(join(dir, entry.name), "utf8"),
      )?.[1];
      if (declared)
        found.set(
          `/${[...trail, ...(slug === "index" ? [] : [slug])].join("/")}`,
          declared.trim(),
        );
    }
  }
  return found;
}

function markdownFiles(dir: string): number {
  return readdirSync(dir, { withFileTypes: true }).reduce(
    (count, entry) =>
      count +
      (entry.isDirectory()
        ? markdownFiles(join(dir, entry.name))
        : Number(entry.name.endsWith(".md"))),
    0,
  );
}

/**
 * Whether a page still shows a `{{name}}` the build was meant to fill. The
 * version these pages quote is passed in at build time, so braces in the
 * output mean the build ran without its `--var` or against a generator that
 * does not know the flag — and a reader would see them while every other
 * assertion here stayed green.
 */
function unfilled(html: string): boolean {
  return /(?<!\$)\{\{\s*[a-z][a-z0-9-]*\s*\}\}/i.test(html);
}

type Page = { url: string; file: string };

function pages(dir: string, trail: string[] = []): Page[] {
  const found: Page[] = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.isDirectory())
      found.push(...pages(join(dir, entry.name), [...trail, entry.name]));
    else if (entry.name === "index.html")
      found.push({ url: `/${trail.join("/")}`, file: join(dir, entry.name) });
  }
  return found;
}

/**
 * The shape this test reads out of the registry TOML the generator ships: only
 * the per-site table, reached through whichever way the import wraps it.
 */
type SiteRegistry = {
  site?: Record<string, Record<string, string> | undefined>;
};

const registry = (await import(
  join(SITE, "node_modules", "@powderworks", "docs", "powderworks.toml")
)) as SiteRegistry & { default?: SiteRegistry };
const named = /--site (\S+)/.exec(script)?.[1];
const shared = (registry.default ?? registry).site?.[named ?? ""] ?? {};

const built = run.exitCode === 0 ? pages(out) : [];
const overrides = tabTitlesByEmittedUrl(join(SITE, "content"));
if (shared["tab-title"]) overrides.set("/", shared["tab-title"]);

describe("rendered titles", () => {
  test("the real build command succeeds", () => {
    expect(run.stderr.toString()).toBe("");
    expect(run.exitCode).toBe(0);
  });

  test("the build emitted a page for every markdown file", () => {
    expect(built.length).toBe(markdownFiles(join(SITE, "content")));
  });

  test("no page ships an unfilled variable", () => {
    const showing = built
      .filter(({ file }) => unfilled(readFileSync(file, "utf8")))
      .map(({ url }) => url);
    expect(showing).toEqual([]);
  });

  test("no name is set in its own face inside code", () => {
    const inside = built.flatMap(({ url, file }) =>
      [...readFileSync(file, "utf8").matchAll(/<(pre|code)\b[\s\S]*?<\/\1>/g)]
        .filter((block) => block[0].includes('class="wordmark'))
        .map(() => url),
    );
    expect(inside).toEqual([]);
  });

  test("the site's own name is set in its own face", () => {
    const html = built.map(({ file }) => readFileSync(file, "utf8")).join("");
    expect(new RegExp(`class="wordmark[^"]*">${SITE_NAME}<`).test(html)).toBe(
      true,
    );
  });

  for (const { url, file } of built.sort((a, b) =>
    a.url.localeCompare(b.url),
  )) {
    test(`${url} names itself in the tab`, () => {
      const html = readFileSync(file, "utf8");
      const title = /<title>([\s\S]*?)<\/title>/.exec(html)?.[1] ?? "";
      const heading = (
        /<h1[^>]*>([\s\S]*?)<\/h1>/.exec(html)?.[1] ?? ""
      ).replace(/<[^>]*>/g, "");

      expect(title).not.toBe("");
      expect(title).not.toContain("undefined");
      expect(heading).not.toBe("");

      if (overrides.has(url)) {
        expect(title).toBe(overrides.get(url));
        return;
      }
      const [name, ...rest] = title.split(" — ");
      expect(name).toBe(heading);
      expect(rest.join(" — ")).toBe(url === "/" ? "" : SITE_NAME);
    });
  }
});
