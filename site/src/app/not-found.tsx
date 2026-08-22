import Link from "next/link";

const links = [
  { href: "/", label: "Home" },
  { href: "/docs", label: "Documentation index" },
  { href: "/docs/getting-started", label: "Getting started" },
  { href: "/sitemap.xml", label: "Sitemap" },
  {
    href: "https://github.com/PowderworksCode/straitjacket",
    label: "GitHub repository",
  },
];

export default function NotFound() {
  return (
    <main className="mx-auto flex w-full max-w-6xl flex-1 flex-col px-6 py-16">
      <h1 className="text-3xl font-bold tracking-tight">Page not found</h1>
      <p className="mt-3 max-w-xl text-fd-muted-foreground">
        There is nothing at this address. Everything the site offers is
        reachable from the places below.
      </p>
      <ul className="mt-6 flex list-disc flex-col gap-2 pl-5 text-fd-muted-foreground">
        {links.map((link) => (
          <li key={link.href}>
            <Link
              href={link.href}
              className="underline hover:text-fd-foreground"
            >
              {link.label}
            </Link>
          </li>
        ))}
      </ul>
    </main>
  );
}
