// Every content page must carry title and description frontmatter: the
// generator turns them into <title>, the meta description, and the entry on
// its section index.
import { describe, expect, test } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

const CONTENT_DIR = join(import.meta.dir, "..", "content");

function markdownFiles(dir) {
  const out = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) out.push(...markdownFiles(full));
    else if (entry.name.endsWith(".md")) out.push(full);
  }
  return out;
}

describe("content frontmatter", () => {
  for (const file of markdownFiles(CONTENT_DIR)) {
    const rel = file.slice(CONTENT_DIR.length + 1);
    test(`${rel} declares title and description`, () => {
      const text = readFileSync(file, "utf8");
      const match = /^---\n([\s\S]*?)\n---/.exec(text);
      expect(match).not.toBeNull();
      const frontmatter = match[1];
      expect(frontmatter).toMatch(/^title: \S/m);
      expect(frontmatter).toMatch(/^description: \S/m);
    });
  }
});
