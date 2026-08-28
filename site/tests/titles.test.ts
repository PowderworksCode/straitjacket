// The browser tab is the one piece of a page no test could see before: the
// frontmatter suite reads sources, and every rendered <title> said "undefined"
// while the suite stayed green. So this builds the site and reads the output.
//
// It runs the real build command out of package.json, pointed at a temporary
// directory. Twice this suite restated that command's flags instead, and twice
// they drifted, leaving it green against a site configured unlike the published
// one. A tab and its heading come from one field by two paths, so asserting
// they agree catches either path dropping it; a page declaring tab-title must
// get exactly what it asked for instead.
import { afterAll, describe, expect, test } from "bun:test";
import { mkdtempSync, readdirSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const SITE = join(import.meta.dir, "..");
const SITE_NAME = "Straitjacket";
const out = mkdtempSync(join(tmpdir(), "straitjacket-titles-"));
afterAll(() => rmSync(out, { recursive: true, force: true }));

const script = JSON.parse(readFileSync(join(SITE, "package.json"), "utf8")).scripts.build;
const run = Bun.spawnSync(
  ["sh", "-c", script.replace("rm -rf out && ", "").replace("--out out", `--out '${out}'`)],
  {
    cwd: SITE,
    env: { ...process.env, PATH: `${join(SITE, "node_modules", ".bin")}:${process.env.PATH}` },
  },
);

function tabTitlesByEmittedUrl(dir, trail = []) {
  const found = new Map();
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.isDirectory()) {
      for (const [url, value] of tabTitlesByEmittedUrl(join(dir, entry.name), [...trail, entry.name]))
        found.set(url, value);
    } else if (entry.name.endsWith(".md")) {
      const slug = entry.name.slice(0, -3);
      const declared = /^tab-title:\s*(.+)$/m.exec(readFileSync(join(dir, entry.name), "utf8"));
      if (declared)
        found.set("/" + [...trail, ...(slug === "index" ? [] : [slug])].join("/"), declared[1].trim());
    }
  }
  return found;
}

function pages(dir, trail = []) {
  const found = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.isDirectory()) found.push(...pages(join(dir, entry.name), [...trail, entry.name]));
    else if (entry.name === "index.html") found.push({ url: "/" + trail.join("/"), file: join(dir, entry.name) });
  }
  return found;
}

const built = run.exitCode === 0 ? pages(out) : [];
const overrides = tabTitlesByEmittedUrl(join(SITE, "content"));

describe("rendered titles", () => {
  test("the real build command succeeds", () => {
    expect(run.stderr.toString()).toBe("");
    expect(run.exitCode).toBe(0);
  });

  test("the build emitted a page for every section and leaf", () => {
    expect(built.length).toBeGreaterThan(15);
  });

  test("no name is set in its own face inside code", () => {
    const inside = built.flatMap(({ url, file }) =>
      [...readFileSync(file, "utf8").matchAll(/<(pre|code)\b[\s\S]*?<\/\1>/g)]
        .filter((block) => block[0].includes('class="wordmark'))
        .map(() => url));
    expect(inside).toEqual([]);
  });

  test("the names given to the build are set where they appear", () => {
    const named = [...script.matchAll(/--wordmark (?:"([^"]+)"|(\S+))/g)].map((m) => m[1] ?? m[2]);
    expect(named.length).toBeGreaterThan(0);
    const html = built.map(({ file }) => readFileSync(file, "utf8")).join("");
    const set = named.filter((name) =>
      html.includes(`>${name}<`) || new RegExp(`class="wordmark[^"]*">${name.split(/\s+/)[0]}`).test(html));
    expect(set.length).toBeGreaterThan(0);
  });

  for (const { url, file } of built.sort((a, b) => a.url.localeCompare(b.url))) {
    test(`${url} names itself in the tab`, () => {
      const html = readFileSync(file, "utf8");
      const title = /<title>([\s\S]*?)<\/title>/.exec(html)?.[1] ?? "";
      const heading = (/<h1[^>]*>([\s\S]*?)<\/h1>/.exec(html)?.[1] ?? "").replace(/<[^>]*>/g, "");

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
