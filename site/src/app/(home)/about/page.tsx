import type { Metadata } from "next";
import Link from "next/link";

export const metadata: Metadata = {
  title: "About",
  description:
    "What Straitjacket is, who builds it, and why it exists.",
};

export default function AboutPage() {
  return (
    <main className="mx-auto w-full max-w-3xl flex-1 px-6 py-16">
      <h1 className="text-3xl font-bold tracking-tight">About Straitjacket</h1>
      <div className="mt-6 flex flex-col gap-4 text-fd-muted-foreground">
        <p>
          Straitjacket is a fast, deterministic scanner that flags the weird
          code and text LLMs produce: emoji glyphs in source files, hardcoded
          colors that should be theme tokens, sprawling thousand-line files,
      logic indented past the depth a reader can hold, and deferred-work
          markers like TODO left behind in comments. It is one static Rust
          binary with no runtime dependencies, so it drops into any
          repository&rsquo;s CI regardless of language or stack.
        </p>
        <p>
          Every rule is lexical — it reads patterns off the bytes rather than
          parsing the language — which is exactly why it works everywhere and
          exactly what it cannot do: judge semantics. The trade is deliberate,
          and the reasoning is written down in{" "}
          <Link href="/docs/explanation" className="underline">
            the explanation docs
          </Link>{" "}
          and in{" "}
          <Link href="/docs/about/philosophy" className="underline">
            the project philosophy
          </Link>
          .
        </p>
        <p>
          Straitjacket is built by{" "}
          <a
            className="underline"
            href="https://powderworks.dev"
          >
            Powderworks
          </a>
          , an independent workshop of open-source developer tools maintained by
          Zack Maril. The code is MIT-licensed and lives on{" "}
          <a
            className="underline"
            href="https://github.com/PowderworksCode/straitjacket"
          >
            GitHub
          </a>
          , where releases, checksums, and the full commit history are public.
        </p>
        <p>
          If you want to shape what it catches — a new smell, a false positive
          you should not have to live with — the{" "}
          <Link href="/docs/about/contributing" className="underline">
            contributing guide
          </Link>{" "}
          is the way in.
        </p>
      </div>
    </main>
  );
}
