import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Contact",
  description:
    "How to reach the Straitjacket maintainers: bug reports, rule proposals, and security issues.",
};

const channels = [
  {
    title: "Bug reports and feature requests",
    body: "Open an issue on the repository. Include the version (straitjacket --version), the command you ran, and a minimal file that reproduces the finding.",
    href: "https://github.com/PowderworksCode/straitjacket/issues",
    label: "GitHub Issues",
  },
  {
    title: "Proposing a new rule",
    body: "New smells start as issues too — describe the pattern, show real examples of it, and say why it signals machine-written content. See the contributing guide for what makes a rule land.",
    href: "https://github.com/PowderworksCode/straitjacket/blob/main/CONTRIBUTING.md",
    label: "Contributing guide",
  },
  {
    title: "Security problems",
    body: "Anything exploitable — crashes on hostile input, path traversal in the walker — should be reported privately via GitHub Security Advisories rather than a public issue.",
    href: "https://github.com/PowderworksCode/straitjacket/security/advisories/new",
    label: "Private security advisory",
  },
];

export default function ContactPage() {
  return (
    <main className="mx-auto w-full max-w-3xl flex-1 px-6 py-16">
      <h1 className="text-3xl font-bold tracking-tight">Contact</h1>
      <p className="mt-4 text-fd-muted-foreground">
        Straitjacket has no support inbox; GitHub is the channel. Everything
        happens in public except security reports, which have a private route.
      </p>
      <div className="mt-8 flex flex-col gap-6">
        {channels.map((channel) => (
          <section
            key={channel.title}
            className="rounded-xl border bg-fd-card p-5"
          >
            <h2 className="font-semibold">{channel.title}</h2>
            <p className="mt-2 text-sm text-fd-muted-foreground">
              {channel.body}{" "}
              <a className="underline" href={channel.href}>
                {channel.label}
              </a>
              .
            </p>
          </section>
        ))}
      </div>
    </main>
  );
}
