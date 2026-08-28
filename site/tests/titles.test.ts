// The browser tab is the one piece of a page no test could see before: the
// frontmatter suite reads sources, and every rendered <title> said "undefined"
// while the suite stayed green. So this builds the site and reads the output.
//
// A page's <title> and its <h1> come from one frontmatter field, by separate
// paths through the generator. Asserting they agree catches either path
// dropping the title, which is how the tab broke.
import { afterAll, describe, expect, test } from "bun:test";
import { build } from "powderworks-docs/src/build.mjs";
import { mkdtempSync, readdirSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const SITE_NAME = "Straitjacket";
const out = mkdtempSync(join(tmpdir(), "straitjacket-titles-"));
afterAll(() => rmSync(out, { recursive: true, force: true }));

await build(join(import.meta.dir, "..", "content"), out, {
  siteUrl: "https://straitjacket.dev",
  name: SITE_NAME,
  description: "A secret scanner, but for slop.",
  github: "PowderworksCode/straitjacket",
});

function pages(dir, trail = []) {
  const found = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.isDirectory()) found.push(...pages(join(dir, entry.name), [...trail, entry.name]));
    else if (entry.name === "index.html") found.push({ url: "/" + trail.join("/"), file: join(dir, entry.name) });
  }
  return found;
}

const built = pages(out);

describe("rendered titles", () => {
  test("the build emitted a page for every section and leaf", () => {
    expect(built.length).toBeGreaterThan(15);
  });

  for (const { url, file } of built.sort((a, b) => a.url.localeCompare(b.url))) {
    test(`${url} names itself in the tab`, () => {
      const html = readFileSync(file, "utf8");
      const title = /<title>([\s\S]*?)<\/title>/.exec(html)?.[1] ?? "";
      const heading = /<h1[^>]*>([\s\S]*?)<\/h1>/.exec(html)?.[1] ?? "";

      expect(title).not.toBe("");
      expect(title).not.toContain("undefined");
      expect(heading).not.toBe("");

      const [name, ...rest] = title.split(" — ");
      expect(name).toBe(heading);
      expect(rest.join(" — ")).toBe(url === "/" ? "" : SITE_NAME);
    });
  }
});
