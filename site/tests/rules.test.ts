// Checks the documentation against content/rules.json, which is exported from
// the scanner by scripts/rules-manifest.sh. The site once documented six rules
// the binary had never carried; these tests are what makes that a failing
// build rather than something a reader discovers.
//
// The rules reference is the one page that has to list every rule. The README
// points at it rather than repeating it, so it is checked like any other page:
// what it names must exist, but it need not name everything.
import { describe, expect, test } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

type Manifest = {
  schema: string;
  version: string;
  rules: { id: string; summary: string; default_enabled: boolean }[];
  removed: string[];
  defaults: Record<string, number | boolean>;
};

const ROOT = join(import.meta.dir, "..");
const DOCS_DIR = join(ROOT, "content");
const RULES_REFERENCE = join(DOCS_DIR, "reference", "rules.md");
const README = join(ROOT, "..", "README.md");

const manifest: Manifest = JSON.parse(
  readFileSync(join(ROOT, "content", "rules.json"), "utf8"),
);

const liveIds = new Set(manifest.rules.map((rule) => rule.id));
const removedIds = new Set(manifest.removed);

function mdxFiles(dir: string): string[] {
  const out: string[] = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) out.push(...mdxFiles(full));
    else if (entry.name.endsWith(".md")) out.push(full);
  }
  return out;
}

const pages = [
  ...mdxFiles(DOCS_DIR).map((path) => ({
    rel: path.slice(DOCS_DIR.length + 1),
    text: readFileSync(path, "utf8"),
  })),
  { rel: "README.md", text: readFileSync(README, "utf8") },
];

/**
 * Rule ids the prose actually asserts are rules, rather than every backticked
 * token that happens to look like one. Three shapes carry that claim:
 * a `[rule]` tag in sample scanner output, a `--only`/`--skip` argument, and an
 * `only`/`skip` list in a TOML or YAML config block.
 */
function claimedRuleIds(text: string): Map<string, string> {
  const claims = new Map<string, string>();
  const add = (id: string, context: string) => {
    if (!claims.has(id)) claims.set(id, context);
  };

  for (const block of text.matchAll(/```[a-z]*\n([\s\S]*?)```/g)) {
    const body = block[1] ?? "";
    for (const hit of body.matchAll(/\[([a-z][a-z0-9-]*)\]\s{2}/g)) {
      const id = hit[1];
      if (id) add(id, `sample output [${id}]`);
    }
  }

  for (const hit of text.matchAll(/--(?:only|skip)[= ]+"?([a-z0-9,-]+)"?/g)) {
    for (const id of (hit[1] ?? "").split(",").filter(Boolean)) {
      add(id, `--only/--skip ${id}`);
    }
  }

  for (const hit of text.matchAll(
    /^\s*(?:only|skip)\s*[:=]\s*\[([^\]]*)\]/gm,
  )) {
    for (const raw of (hit[1] ?? "").split(",")) {
      const id = raw.trim().replace(/^["']|["']$/g, "");
      if (/^[a-z][a-z0-9-]*$/.test(id)) add(id, `config list ${id}`);
    }
  }

  return claims;
}

describe("rules manifest", () => {
  test("is the schema these tests understand", () => {
    expect(manifest.schema).toBe("straitjacket.rules/1");
    expect(manifest.rules.length).toBeGreaterThan(0);
  });

  test("no rule is both live and withdrawn", () => {
    for (const id of removedIds) expect(liveIds.has(id)).toBe(false);
  });
});

describe("every rule is documented", () => {
  const reference = readFileSync(RULES_REFERENCE, "utf8");

  for (const rule of manifest.rules) {
    test(`${rule.id} appears in the rules reference`, () => {
      expect(reference).toContain(`\`${rule.id}\``);
    });
  }

  test("the reference marks the opt-in rules as opt-in", () => {
    for (const rule of manifest.rules.filter((r) => !r.default_enabled)) {
      const row = reference
        .split("\n")
        .find((line) => line.startsWith(`| \`${rule.id}\``));
      expect(row, `no table row for ${rule.id}`).toBeDefined();
      expect(row).toContain("opt-in");
    }
  });
});

describe("no page claims a rule that does not exist", () => {
  for (const page of pages) {
    const claims = claimedRuleIds(page.text);
    if (claims.size === 0) continue;

    test(`${page.rel}`, () => {
      const bogus: string[] = [];
      for (const [id, context] of claims) {
        if (liveIds.has(id)) continue;
        const why = removedIds.has(id)
          ? `${id} was withdrawn from Straitjacket`
          : `${id} is not a Straitjacket rule`;
        bogus.push(`${why} (${context})`);
      }
      expect(bogus).toEqual([]);
    });
  }
});

describe("no page mentions a withdrawn rule at all", () => {
  for (const page of pages) {
    test(`${page.rel}`, () => {
      const mentioned = [...removedIds].filter((id) =>
        page.text.includes(`\`${id}\``),
      );
      expect(mentioned).toEqual([]);
    });
  }
});

describe("documented defaults match the binary", () => {
  const reference = readFileSync(RULES_REFERENCE, "utf8");

  /**
   * Every "default N" the prose states, paired with the budget it is talking
   * about. Matching runs over whitespace-collapsed text because these phrases
   * routinely wrap across lines in the MDX source.
   */
  function quotedDefaults(text: string) {
    const flat = text.replace(/\s+/g, " ");
    const found: { budget: "nesting" | "lines"; value: number }[] = [];
    const nearest = (context: string, pattern: RegExp) => {
      let at = -1;
      for (const hit of context.matchAll(pattern)) at = hit.index ?? at;
      return at;
    };
    for (const hit of flat.matchAll(
      /\*{0,2}[Dd]efault\*{0,2}\s*\*{0,2}(\d+)/g,
    )) {
      const from = Math.max(0, (hit.index ?? 0) - 140);
      const context = flat.slice(from, hit.index ?? 0);
      const nesting = nearest(
        context,
        /nesting budget|--max-nesting|nesting/gi,
      );
      const lines = nearest(
        context,
        /line budget|--max-lines|lines per file|long a file/gi,
      );
      if (nesting < 0 && lines < 0) continue;
      found.push({
        budget: nesting > lines ? "nesting" : "lines",
        value: Number(hit[1]),
      });
    }
    return found;
  }

  test("the defaults table quotes the exported numbers", () => {
    const table = reference.slice(reference.indexOf("## Defaults at a glance"));
    expect(table).toContain(`| ${manifest.defaults["max-lines"]} |`);
    expect(table).toContain(`| ${manifest.defaults["max-nesting"]} |`);
  });

  test("the rules reference states both budgets at least once", () => {
    const stated = quotedDefaults(reference).map((d) => d.budget);
    expect(stated).toContain("nesting");
    expect(stated).toContain("lines");
  });

  for (const page of pages) {
    const quoted = quotedDefaults(page.text);
    if (quoted.length === 0) continue;

    test(`${page.rel} quotes no stale budget`, () => {
      const stale = quoted
        .filter(
          (d) =>
            d.value !==
            manifest.defaults[
              d.budget === "nesting" ? "max-nesting" : "max-lines"
            ],
        )
        .map((d) => `${d.budget} default stated as ${d.value}`);
      expect(stale).toEqual([]);
    });
  }
});
