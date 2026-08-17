// straitjacket-allow-file:color — the sample terminal output quotes a color
// finding, so the literal is the thing being demonstrated.
// straitjacket-allow-file:emoji — likewise for the emoji finding below.
import Link from "next/link";

const RULES = [
  {
    title: "Emoji & AI tells",
    body: "Emoji glyphs in code, comments, strings, and Markdown — one of the most reliable giveaways that a machine wrote it.",
  },
  {
    title: "Hardcoded colors",
    body: "Raw hex and CSS color functions (rgb, hsl, oklch, …) that should be a theme token, not sprinkled inline.",
  },
  {
    title: "Oversized files",
    body: "The 1,500-line monsters that pass review one screen at a time and sneak up on you. Tunable, off with a flag.",
  },
  {
    title: "Deep nesting",
    body: "Logic indented past the depth budget, read straight off the indentation — so it works in any language without a parser.",
  },
  {
    title: "Stray TODOs",
    body: "TODO, TBD, FIXME and WIP markers left behind in comments. Do the work, or track it somewhere the repository can see.",
  },
  {
    title: "Design drift",
    body: "Inline SVG, literal font stacks, and ad-hoc transitions and keyframes — the details that quietly escape a design system.",
  },
];

const INSTALL =
  "curl -fsSL https://raw.githubusercontent.com/PowderworksCode/straitjacket/main/install.sh | sh";

export default function HomePage() {
  return (
    <main className="flex flex-1 flex-col">
      <section className="mx-auto grid w-full max-w-6xl grid-cols-1 items-center gap-10 px-6 py-10 md:grid-cols-[minmax(0,1fr)_1.4fr] md:py-14">
        <figure className="mx-auto w-full max-w-[200px] md:justify-self-center lg:max-w-[240px]">
          <img
            src="/strait-waistcoat.jpg"
            alt="Engraving of a patient restrained in a strait-waistcoat"
            className="w-full rounded-xl border shadow-sm"
            width={1148}
            height={1814}
          />
          <figcaption className="mt-3 text-center text-xs text-fd-muted-foreground">
            Insane patient in a strait-waistcoat. Wellcome Collection
            (L0011301),{" "}
            <a
              className="underline"
              href="https://creativecommons.org/licenses/by/4.0"
            >
              CC BY 4.0
            </a>
            .
          </figcaption>
        </figure>

        <div className="flex flex-col items-start text-left">
          <h1 className="text-4xl font-bold tracking-tight sm:text-5xl">
            Straitjacket
          </h1>
          <p className="mt-3 text-xl font-medium text-fd-foreground">
            A secret scanner, but for slop.
          </p>
          <p className="mt-4 text-fd-muted-foreground">
            Straitjacket is a fast, deterministic scanner that flags the weird
            code and text LLMs produce. It sweeps your files against a set of
            snobby-but-configurable rules and flags anything it finds — one
            static Rust binary, no runtime, so it drops into any repo's CI
            regardless of language or stack.
          </p>

          <div className="mt-8 flex flex-wrap items-center gap-3">
            <Link
              href="/docs/tutorials/getting-started"
              className="rounded-full bg-fd-primary px-6 py-2.5 text-sm font-medium text-fd-primary-foreground transition-colors hover:bg-fd-primary/90"
            >
              Get started
            </Link>
            <Link
              href="/docs"
              className="rounded-full border px-6 py-2.5 text-sm font-medium transition-colors hover:bg-fd-accent"
            >
              Read the docs
            </Link>
            <a
              href="https://github.com/PowderworksCode/straitjacket"
              className="rounded-full border px-6 py-2.5 text-sm font-medium transition-colors hover:bg-fd-accent"
            >
              GitHub
            </a>
          </div>

          <pre className="mt-8 w-full overflow-x-auto rounded-lg border bg-fd-card p-4 text-left text-sm text-fd-muted-foreground">
            <code>{INSTALL}</code>
          </pre>
          <p className="mt-2 text-xs text-fd-muted-foreground">
            Or build from source with{" "}
            <code className="text-xs">cargo install straitjacket</code>.
          </p>
        </div>
      </section>

      <section className="mx-auto w-full max-w-6xl px-6 py-8">
        <h2 className="text-2xl font-semibold tracking-tight">
          What it catches
        </h2>
        <p className="mt-2 max-w-2xl text-fd-muted-foreground">
          Everything is on by default — Straitjacket runs at its max, and you
          ratchet down with <code className="text-sm">--skip</code>. Each rule
          only looks at the file types where it makes sense. The one exception
          is <code className="text-sm">no-comments</code>, a mode you opt into.
        </p>
        <div className="mt-8 grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {RULES.map((rule) => (
            <div
              key={rule.title}
              className="rounded-xl border bg-fd-card p-5 transition-colors hover:bg-fd-accent/50"
            >
              <h3 className="font-semibold">{rule.title}</h3>
              <p className="mt-2 text-sm text-fd-muted-foreground">
                {rule.body}
              </p>
            </div>
          ))}
        </div>
      </section>

      <section className="mx-auto w-full max-w-6xl px-6 py-12">
        <div className="grid grid-cols-1 items-center gap-10 lg:grid-cols-2">
          <div>
            <h2 className="text-2xl font-semibold tracking-tight">
              One command. Every rule.
            </h2>
            <p className="mt-4 text-fd-muted-foreground">
              Run <code className="text-sm">straitjacket</code> at the root of
              any project. It honors your{" "}
              <code className="text-sm">.gitignore</code>, prints one line per
              finding as{" "}
              <code className="text-sm">path:line:col [rule] matched</code>, and
              exits non-zero on any error — so CI fails the moment slop lands.
            </p>
            <p className="mt-4 text-fd-muted-foreground">
              No config to write, no toolchain to install. Suppress a false
              positive on one line with{" "}
              <code className="text-sm">straitjacket-allow</code>, or a whole
              file with <code className="text-sm">straitjacket-allow-file</code>
              .
            </p>
          </div>
          <pre className="overflow-x-auto rounded-xl border bg-fd-card p-5 text-sm leading-relaxed">
            <code>
              <span className="text-fd-muted-foreground">$ </span>straitjacket
              {"\n\n"}
              src/theme.ts:42:7 <span className="text-fd-primary">[color]</span>{" "}
              #1e1e1e{"\n"}
              src/icons/Logo.tsx:12:5{" "}
              <span className="text-fd-primary">[inline-svg]</span> &lt;svg
              {"\n"}
              docs/setup.md:3:1 <span className="text-fd-primary">[emoji]</span>{" "}
              🚀{"\n"}
              src/api/handlers.ts:1:1{" "}
              <span className="text-fd-primary">[file-size]</span> 2214 lines
              {"\n"}
              src/worker.ts:88:31{" "}
              <span className="text-fd-primary">[deep-nesting]</span> nesting
              depth 9{"\n\n"}
              <span className="text-fd-muted-foreground">
                straitjacket: 5 error(s), 0 warning(s) across 128 file(s); 0
                suppressed
              </span>
            </code>
          </pre>
        </div>
      </section>

      <section className="mx-auto w-full max-w-6xl px-6 py-16">
        <div className="flex flex-col items-center gap-6 rounded-2xl border bg-fd-card px-6 py-12 text-center">
          <h2 className="text-2xl font-semibold tracking-tight">
            Put your slop in a Straitjacket.
          </h2>
          <p className="max-w-xl text-fd-muted-foreground">
            Encode your taste as deterministic checks and run them across
            everything an LLM writes — so you never have to go "Yuck!" by hand
            again.
          </p>
          <div className="flex flex-wrap items-center justify-center gap-3">
            <Link
              href="/docs/tutorials/getting-started"
              className="rounded-full bg-fd-primary px-6 py-2.5 text-sm font-medium text-fd-primary-foreground transition-colors hover:bg-fd-primary/90"
            >
              Get started
            </Link>
            <Link
              href="/docs/reference/rules"
              className="rounded-full border px-6 py-2.5 text-sm font-medium transition-colors hover:bg-fd-accent"
            >
              Browse the rules
            </Link>
          </div>
        </div>
      </section>
    </main>
  );
}
