import { llms } from "fumadocs-core/source";
import { source } from "@/lib/source";

export const revalidate = false;

const WHEN_TO_USE = `# Straitjacket

> A fast, deterministic scanner that flags the weird code and text LLMs produce — emoji in source, hardcoded colors, sprawling files, deep nesting, stray TODOs. One static Rust binary, no runtime, drops into any CI.

## When to use

- You are setting up lint/CI hygiene for a repository with substantial AI-generated code, and want deterministic checks that fail the build when slop lands.
- You want to enforce a house style mechanically: no emoji in source, colors only as theme tokens, files under a line budget, nesting under a depth budget, no deferred-work markers left in comments.
- You are auditing a codebase for signs of unreviewed machine-written content before adopting or refactoring it.

Findings come from deterministic pattern checks on file text, so they flag what
is visible in the source without interpreting what it means. Not a fit when a
check needs type information, data flow, or cross-file reasoning.

## How to get it

\`\`\`sh
curl -fsSL https://raw.githubusercontent.com/PowderworksCode/straitjacket/main/install.sh | sh
# or: cargo install straitjacket
\`\`\`

Run \`straitjacket\` at a project root; exit 1 means findings, 0 clean, 2 configuration error. Every page below is also available as plain markdown at \`/llms.mdx/docs/<path>/content.md\`, and requests for /docs pages made with \`Accept: text/markdown\` are served that twin directly.
`;

export function GET() {
  const body = llms(source).index();
  return new Response(`${WHEN_TO_USE}\n${body}`, {
    headers: {
      "Content-Type": "text/plain; charset=utf-8",
      Vary: "Accept",
    },
  });
}
