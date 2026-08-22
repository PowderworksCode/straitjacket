import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Privacy",
  description:
    "What straitjacket.dev collects: nothing. No analytics, no cookies, no accounts.",
};

export default function PrivacyPage() {
  return (
    <main className="mx-auto w-full max-w-3xl flex-1 px-6 py-16">
      <h1 className="text-3xl font-bold tracking-tight">Privacy</h1>
      <div className="mt-6 flex flex-col gap-4 text-fd-muted-foreground">
        <p>
          This site is a static export of plain HTML, CSS, and JavaScript. It
          has no server-side application: pages are files served from Cloudflare&rsquo;s
          CDN. There is no database, no account system, and nothing to sign in
          to.
        </p>
        <p>
          The site sets no cookies, runs no analytics, embeds no third-party
          trackers, and loads no fonts or scripts from advertising networks.
          The one font used is self-hosted with the page. Nothing you read
          here is associated with an identity, because the site never asks for
          one.
        </p>
        <p>
          Like any site on Cloudflare, request logs exist at the
          infrastructure level (IP address, requested URL, timestamp) for
          security and abuse prevention. Those logs are operated by Cloudflare,
          retained per their standard policy, and are not combined with any
          profile — there is no profile to combine them with.
        </p>
        <p>
          The scanner itself is a local binary: it reads only the files you
          point it at, makes no network requests while scanning, and reports
          findings to your terminal. If you pipe its output somewhere, that is
          your doing, not the tool&rsquo;s.
        </p>
        <p>
          Questions about this page? The{" "}
          <a
            className="underline"
            href="https://github.com/PowderworksCode/straitjacket/issues"
          >
            issue tracker
          </a>{" "}
          is the right place.
        </p>
      </div>
    </main>
  );
}
